//! Client library for remote CUDA execution.
//! Exposes Args, handle_run, handle_status, config, and utils for testing.

pub mod config;
pub mod discovery;
pub mod interactive;
mod utils;

pub use utils::{gather_files_recursive, read_ignore};

use clap::Parser;
use colored::*;
use common::compute::ComputeRequest;
use common::compute::cuda_executor_client::CudaExecutorClient;
use std::collections::HashMap;
use std::path::PathBuf;

const MAX_WORKSPACE_SIZE_MB: usize = 50;

#[derive(Parser, Debug)]
#[command(name = "ferris-run", about = "Remote CUDA Execution Tool")]
pub enum Args {
    /// Execute a CUDA file on the remote host
    Run {
        /// Paths to .cu/.cuh/.h/.cpp files or directories containing them
        /// Supports multiple files and directories
        inputs: Vec<PathBuf>,

        /// Remote host address (e.g., http://192.168.1.50:50051)
        #[arg(short, long, env = "FERRIS_SERVER")]
        server: Option<String>,

        /// Extra flags for nvcc (e.g., "-arch=sm_80")
        #[arg(short, long)]
        flags: Vec<String>,

        #[arg(short, long, env = "FERRIS_AUTH_TOKEN")]
        token: String,
    },

    /// Check the status of the remote GPU
    Status {
        /// Remote host address (e.g., http://192.168.1.50:50051)
        #[arg(short, long, env = "FERRIS_SERVER")]
        server: Option<String>,

        #[arg(short, long, env = "FERRIS_AUTH_TOKEN")]
        token: String,
    },

    /// Discover ferris-compute hosts on the local network
    Discover,
}

/// Resolves the server address using the priority chain:
/// CLI arg > FERRIS_SERVER env > ~/.ferris-compute/config.toml > None
pub fn resolve_server(cli_server: Option<String>) -> Option<String> {
    if let Some(s) = cli_server {
        return Some(s);
    }
    config::resolve_server_from_config()
}

/// Execute CUDA code on the remote host.
pub async fn handle_run(
    inputs: Vec<PathBuf>,
    server: &str,
    flags: Vec<String>,
    token: String,
) -> Result<(), Box<dyn std::error::Error>> {
    if inputs.is_empty() {
        return Err("No input files provided.".into());
    }

    let mut files = HashMap::new();

    // 1. Establish the Global Anchor (Base Directory)
    // We use the parent of the FIRST input (entry point) as the root of the entire sync.
    let entry_input = inputs[0].canonicalize()?;
    let global_base = if entry_input.is_dir() {
        entry_input.clone()
    } else {
        entry_input
            .parent()
            .ok_or("Invalid base: Could not determine base directory")?
            .to_path_buf()
    };

    let ignore_list = read_ignore(&global_base);

    // 2. Gather files using the fix Global Base
    for input in &inputs {
        let canon_path = input.canonicalize()?;
        gather_files_recursive(&global_base, &canon_path, &mut files, &ignore_list)?;
    }

    // 2. Pre-checks: Ensure we have at least one file and that total size is within limits
    if files.is_empty() {
        eprintln!(
            "{}",
            "❌ Error: No valid CUDA/C++ files found in provided inputs.".red()
        );
        std::process::exit(1);
    }

    let total_size_mb: usize = files.values().map(|content| content.len()).sum::<usize>() / (1024 * 1024);
    if total_size_mb > MAX_WORKSPACE_SIZE_MB {
        eprintln!(
            "{} {} MB (limit: {} MB). Please reduce the number or size of files.",
            "❌ Error: Total workspace size".red(),
            total_size_mb,
            MAX_WORKSPACE_SIZE_MB
        );
        std::process::exit(1);
    }

    // 3. Identify Entry Point (the first path provided by the user)
    let entry_file = entry_input
        .strip_prefix(&global_base)?
        .to_string_lossy()
        .replace('\\', "/");

    // 3. Mode Detection & Acknowledgment
    let is_multi = files.len() > 1;
    let mode_tag = if is_multi {
        format!("📂 {}", "Multi-file Workspace".bold().cyan())
    } else {
        format!("📄 {}", "Single-file Mode".bold().green())
    };

    println!("{} Mode: {}", "🚀".bold(), mode_tag);
    println!("{} Connecting to host at {}...", "📡".bold(), server.cyan());

    // 4. Connect and Prepare Request
    let mut client = CudaExecutorClient::connect(server.to_string()).await?;
    let files_len = files.len();

    // Note: The .proto was updated to use 'map<string, string> files' and 'string entry_point_file'
    let mut request = tonic::Request::new(ComputeRequest {
        files,
        entry_point_file: entry_file.clone(),
        compiler_flags: flags,
    });

    println!(
        "{} Syncing {} files (Entry: {}) to remote GPU...",
        "📤".bold(),
        files_len.to_string().yellow(),
        entry_file.yellow()
    );

    // 5. Inject Auth Token
    let token_value = token.parse::<tonic::metadata::MetadataValue<tonic::metadata::Ascii>>()?;
    request.metadata_mut().insert("x-ferris-token", token_value);

    // 6. Execution Loop
    let response = client.execute_code(request).await;
    match response {
        Ok(res) => {
            let mut stream = res.into_inner();
            while let Some(msg) = stream.message().await? {
                if msg.is_error {
                    eprintln!("{}", msg.output.red());
                } else {
                    println!("{}", msg.output);
                }
            }
        }
        // Authentication error: token not valid
        Err(e) if e.code() == tonic::Code::Unauthenticated => {
            eprintln!(
                "{}\n{}",
                "🛑 Auth Error: Access denied. Check your --token:".red(),
                token.bold().yellow()
            );
        }
        // RPC error: network issue or server error
        Err(e) => {
            eprintln!("{}\n{}", "❌ RPC Error: {}".red(), e.message().magenta());
        }
    }

    config::save_history_entry(server);
    println!("\n{} Execution finished.", "✅".bold().green());
    Ok(())
}

/// Query GPU status from the remote host.
pub async fn handle_status(server: &str, token: String) -> Result<(), Box<dyn std::error::Error>> {
    println!("📡 Querying GPU status from {}...", server);

    // 1. Connect to the remote Host
    let mut client = common::compute::cuda_executor_client::CudaExecutorClient::connect(server.to_string()).await?;

    // 2. Prepare the Request with an empty body
    let mut request = tonic::Request::new(common::compute::Empty {});

    // 3. Inject the Authentication Token into Metadata
    let token_value = token.parse::<tonic::metadata::MetadataValue<tonic::metadata::Ascii>>()?;
    request.metadata_mut().insert("x-ferris-token", token_value);

    // 4. Call the GetGpuStatus method
    match client.get_gpu_status(request).await {
        Ok(response) => {
            let status = response.into_inner();

            println!("\n--- 🖥️  Remote GPU Status ---");
            println!("Model:       {}", status.gpu_name);
            println!("Temperature: {}°C", status.temperature_celsius);
            println!(
                "Memory:      {} / {} MB ({:.1}%)",
                status.memory_used_mb,
                status.memory_total_mb,
                (status.memory_used_mb as f32 / status.memory_total_mb as f32) * 100.0
            );
            println!("Utilization: {}%", status.load_percentage);
            println!("---------------------------\n");
        }
        Err(e) if e.code() == tonic::Code::Unauthenticated => {
            eprintln!("🛑 Auth Error: Access denied. Check your --token or .env file.");
        }
        Err(e) => {
            eprintln!("❌ Failed to retrieve status: {}", e.message());
        }
    }

    config::save_history_entry(server);
    Ok(())
}

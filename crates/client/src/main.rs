/// This code handles the connection, file reading, and the asynchronous loop that listens to the server's stream.
use clap::{Parser};
use colored::*;
use common::compute::ComputeRequest;
use common::compute::cuda_executor_client::CudaExecutorClient;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(name = "ferris-run", about = "Remote CUDA Execution Tool")]
pub enum Args {
    /// Execute a CUDA file on the remote host
    Run {
        /// Paths to .cu/.cuh/.h/.cpp files or directories containing them
        /// Supports multiple files and directories
        inputs: Vec<PathBuf>,

        /// Remote host address (e.g., http://192.168.1.50:50051)
        #[arg(short, long, default_value = "http://[::1]:50051")]
        server: String,

        /// Extra flags for nvcc (e.g., "-arch=sm_80")
        #[arg(short, long)]
        flags: Vec<String>,

        #[arg(short, long, env = "FERRIS_AUTH_TOKEN")]
        token: String,
    },

    /// Check the status of the remote GPU
    Status {
        /// Remote host address (e.g., http://192.168.1.50:50051)
        #[arg(short, long, default_value = "http://[::1]:50051")]
        server: String,

        #[arg(short, long, env = "FERRIS_AUTH_TOKEN")]
        token: String,
    },
}

/// Recursively gathers valid CUDA/C++ files from a path (file or directory)
fn gather_files_recursive(
    base_dir: &Path,
    current_path: &Path,
    files_map: &mut HashMap<String, String>,
) -> Result<(), Box<dyn std::error::Error>> {
    const ALLOWED_EXTENSIONS: [&str; 6] = ["cu", "cuh", "ptx", "cubin", "h", "cpp"];

    if current_path.is_dir() {
        for entry in std::fs::read_dir(current_path)? {
            gather_files_recursive(base_dir, &entry?.path(), files_map)?;
        }
    } else {
        let extension = current_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default()
            .to_lowercase();

        if ALLOWED_EXTENSIONS.contains(&extension.as_str()) {
            let content = std::fs::read_to_string(current_path)
                .map_err(|e| format!("Could not read file {}: {}", current_path.display(), e))?;
            
            // Reconstruct relative path for the Host (e.g., "src/kernel.cu")
            let relative_path = current_path
                .strip_prefix(base_dir)
                .unwrap_or(current_path)
                .to_string_lossy()
                .into_owned();
            
            files_map.insert(relative_path, content);
        }
    }
    Ok(())
}

async fn handle_run(
    inputs: Vec<PathBuf>, 
    server: String, 
    flags: Vec<String>, 
    token: String
) -> Result<(), Box<dyn std::error::Error>> {
    let mut files = HashMap::new();

    // 1. Gather all files from all provided inputs (files or directories)
    for input in &inputs {
        // If input is a file, the "base" is its parent so the file itself is gathered.
        // If input is a directory, the directory itself is the base.
        let base: &Path = if input.is_dir() {
            input.as_path()
        } else {
            input.parent().unwrap_or(Path::new("."))
        };
        gather_files_recursive(base, input, &mut files)?;
    }

    if files.is_empty() {
        eprintln!("{}", "❌ Error: No valid CUDA/C++ files found in provided inputs.".red());
        std::process::exit(1);
    }

    // 2. Identify Entry Point (the first path provided by the user)
    let entry_file = inputs[0]
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // 3. Mode Detection & Acknowledgment
    let is_multi = files.len() > 1;
    let mode_tag = if is_multi {
        format!("📂 {}", "Multi-file Workspace".bold().cyan())
    } else {
        format!("📄 {}", "Single-file Mode".bold().green())
    };

    println!("{} Mode: {}", "🚀".bold(), mode_tag);
    println!(
        "{} Connecting to host at {}...",
        "📡".bold(),
        server.cyan()
    );

    // 4. Connect and Prepare Request
    let mut client = CudaExecutorClient::connect(server.clone()).await?;
    let files_len = files.len();

    // Note: The .proto was updated to use 'map<string, string> files' and 'string entry_point_file'
    let mut request= tonic::Request::new(ComputeRequest {
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

    println!("\n{} Execution finished.", "✅".bold().green());
    Ok(())
}

async fn handle_status(server: String, token: String) -> Result<(), Box<dyn std::error::Error>> {
    println!("📡 Querying GPU status from {}...", server);

    // 1. Connect to the remote Host
    let mut client = common::compute::cuda_executor_client::CudaExecutorClient::connect(server).await?;

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
            println!("Memory:      {} / {} MB ({:.1}%)", 
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

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///////// PRE-FLIGHT CHECKS /////////
    // 1. Load .env file into process env variable
    let _ = dotenvy::dotenv();

    // 2. Parse CLI arguments (Clap handles the env fallback automatically now)
    let args = Args::parse();

    match args {
        Args::Run { inputs, server, flags, token } => {
            handle_run(inputs, server, flags, token).await?
        }
        Args::Status { server, token } => {
            handle_status(server, token).await?
        }
    }

    Ok(())
}

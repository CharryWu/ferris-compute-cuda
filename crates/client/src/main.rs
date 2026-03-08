/// This code handles the connection, file reading, and the asynchronous loop that listens to the server's stream.
use clap::{Parser};
use colored::*;
use common::compute::ComputeRequest;
use common::compute::cuda_executor_client::CudaExecutorClient;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "ferris-run", about = "Remote CUDA Execution Tool")]
pub enum Args {
    /// Execute a CUDA file on the remote host
    Run {
        /// Path to the .cu file
        file: PathBuf,

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


async fn handle_run(file: PathBuf, server: String, flags: Vec<String>, token: String) -> Result<(), Box<dyn std::error::Error>> {
    
    // 3. Read the local CUDA file
    let source_code = std::fs::read_to_string(&file)
        .map_err(|e| format!("Could not read file {}: {}", file.display(), e))?;

    // 4. Validate file extension
    let file_name = file
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    const ALLOWED_EXTENSIONS: [&str; 4] = ["cu", "cuh", "ptx", "cubin"];
    let extension = file
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_lowercase();

    if !ALLOWED_EXTENSIONS.contains(&extension.as_str()) {
        eprintln!(
            "❌ Validation Error: Unsupported file extension '.{}'",
            extension
        );
        eprintln!("Supported types: {:?}", ALLOWED_EXTENSIONS);
        std::process::exit(1);
    }

    ///////// CONNECTION /////////
    println!(
        "{} Connecting to host at {}...",
        "🚀".bold(),
        server.cyan()
    );

    // 1. Connect to the host
    let mut client = CudaExecutorClient::connect(server).await?;

    let mut request = tonic::Request::new(ComputeRequest {
        source_code,
        file_name: file_name.clone(),
        compiler_flags: flags,
    });

    println!(
        "{} Sending {} to remote GPU...",
        "📤".bold(),
        file_name.yellow()
    );

    // Insert the token into metadata
    let token_value = token
        .parse::<tonic::metadata::MetadataValue<tonic::metadata::Ascii>>()?;
    request.metadata_mut().insert("x-ferris-token", token_value);

    ///////// EXECUTION RESULT DISPLAY AND ERROR HANDLING /////////
    let response = client.execute_code(request).await;
    match response {
        Ok(res) => {
            let mut stream = res.into_inner();
            while let Some(msg) = stream.message().await? {
                if msg.is_error {
                    // Print compiler errors or GPU runtime error outputs in red
                    eprintln!("{}", msg.output.red());
                } else {
                    // Print standard output in green/white
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
        Args::Run { file, server, flags, token } => {
            handle_run(file, server, flags, token).await?
        }
        Args::Status { server, token } => {
            handle_status(server, token).await?
        }
    }

    Ok(())
}

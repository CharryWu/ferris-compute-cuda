/// This code handles the connection, file reading, and the asynchronous loop that listens to the server's stream.
use clap::Parser;
use colored::*;
use common::compute::cuda_executor_client::CudaExecutorClient;
use common::compute::ComputeRequest;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about = "Remote CUDA Executor Client")]
struct Args {
    /// Path to the .cu file
    file: PathBuf,

    /// Remote host address (e.g., http://192.168.1.50:50051)
    #[arg(short, long, default_value = "http://[::1]:50051")]
    server: String,

    /// Extra flags for nvcc (e.g., "-arch=sm_80")
    #[arg(short, long)]
    flags: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // 1. Read the local CUDA file
    let source_code = std::fs::read_to_string(&args.file)
        .map_err(|e| format!("Could not read file {}: {}", args.file.display(), e))?;

    let file_name = args.file
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    println!("{} Connecting to host at {}...", "🚀".bold(), args.server.cyan());

    // 2. Connect to the host
    let mut client = CudaExecutorClient::connect(args.server).await?;

    let request = tonic::Request::new(ComputeRequest {
        source_code,
        file_name,
        compiler_flags: args.flags,
    });

    println!("{} Sending {} to remote GPU...", "📤".bold(), file_name.yellow());

    // 3. Receive the stream
    let mut stream = client.execute_code(request).await?.into_inner();

    while let Some(response) = stream.message().await? {
        if response.is_error {
            // Print compiler errors or stderr in red
            eprintln!("{}", response.output.red());
        } else {
            // Print standard output in green/white
            println!("{}", response.output);
        }
    }

    println!("\n{} Execution finished.", "✅".bold().green());

    Ok(())
}
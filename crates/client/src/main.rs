/// This code handles the connection, file reading, and the asynchronous loop that listens to the server's stream.
use clap::Parser;
use colored::*;
use common::compute::ComputeRequest;
use common::compute::cuda_executor_client::CudaExecutorClient;
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

    #[arg(short, long)]
    token: String, // For authentication purpose
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // 1. Read the local CUDA file
    let source_code = std::fs::read_to_string(&args.file)
        .map_err(|e| format!("Could not read file {}: {}", args.file.display(), e))?;

    let file_name = args
        .file
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    const ALLOWED_EXTENSIONS: [&str; 4] = ["cu", "cuh", "ptx", "cubin"];
    let extension = args
        .file
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

    println!(
        "{} Connecting to host at {}...",
        "🚀".bold(),
        args.server.cyan()
    );

    // 2. Connect to the host
    let mut client = CudaExecutorClient::connect(args.server).await?;

    let mut request = tonic::Request::new(ComputeRequest {
        source_code,
        file_name: file_name.clone(),
        compiler_flags: args.flags,
    });

    println!(
        "{} Sending {} to remote GPU...",
        "📤".bold(),
        file_name.yellow()
    );

    // Insert the token into metadata
    let token_value = args
        .token
        .parse::<tonic::metadata::MetadataValue<tonic::metadata::Ascii>>()?;
    request.metadata_mut().insert("x-ferris-token", token_value);

    // 3. Receive the stream
    let response = client.execute_code(request).await;
    match response {
        Ok(res) => {
            let mut stream = res.into_inner();
            while let Some(msg) = stream.message().await? {
                if msg.is_error {
                    // Print compiler errors or stderr in red
                    eprintln!("{}", msg.output.red());
                } else {
                    // Print standard output in green/white
                    println!("{}", msg.output);
                }
            }
        }
        Err(e) if e.code() == tonic::Code::Unauthenticated => {
            eprintln!("🛑 Auth Error: Access denied. Check your --token.");
        }
        Err(e) => {
            eprintln!("❌ RPC Error: {}", e.message());
        }
    }

    println!("\n{} Execution finished.", "✅".bold().green());

    Ok(())
}

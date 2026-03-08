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

    #[arg(short, long, env = "FERRIS_AUTH_TOKEN")]
    token: String, // For authentication purpose
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///////// PRE-FLIGHT CHECKS /////////
    // 1. Load .env file into process env variable
    let _ = dotenvy::dotenv();

    // 2. Parse CLI arguments (Clap handles the env fallback automatically now)
    let args = Args::parse();

    // 3. Read the local CUDA file
    let source_code = std::fs::read_to_string(&args.file)
        .map_err(|e| format!("Could not read file {}: {}", args.file.display(), e))?;

    // 4. Validate file extension
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

    ///////// CONNECTION /////////
    println!(
        "{} Connecting to host at {}...",
        "🚀".bold(),
        args.server.cyan()
    );

    // 1. Connect to the host
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
                args.token.bold().yellow()
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

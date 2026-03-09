mod utils;

use anyhow::Context;
use clap::Parser;
use common::compute::cuda_executor_server::{CudaExecutor, CudaExecutorServer};
use common::compute::{ComputeRequest, ComputeResponse};
use std::path::Path;
use std::time::Duration;
use tokio::fs;
use tokio::process::Command as AsyncCommand;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, transport::Server};

const EXECUTION_TIMEOUT_SECS: u64 = 30; // Timeout for executing compiled binaries

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct HostArgs {
    // Auth token. Priority: CLI Flag > Env Var > .env file
    #[arg(short, long, env = "FERRIS_AUTH_TOKEN")]
    token: String,
}

/// The Interceptor: This function runs BEFORE the service logic.
/// It checks if the 'x-ferris-token' matches our server's secret.
fn check_auth(req: Request<()>, expected_token: String) -> Result<Request<()>, Status> {
    match req.metadata().get("x-ferris-token") {
        Some(token) if token == expected_token.as_str() => {
            // Token matches! Pass the request through to the executor.
            Ok(req)
        }
        _ => {
            // Token missing or wrong. Reject immediately.
            Err(Status::unauthenticated("Invalid or missing auth token"))
        }
    }
}

async fn send_output(tx: &mpsc::Sender<Result<ComputeResponse, Status>>, output: String, is_error: bool) {
    if !output.is_empty() {
        let _ = tx.send(Ok(ComputeResponse { output, is_error })).await;
    }
}

fn u8_to_string(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

/// Drives the full compile-and-run pipeline for a single job.
///
/// Using `?` for setup failures keeps each step flat and readable.
/// Compile/execution results are reported through the stream rather than
/// propagated as errors, so the caller only receives an `Err` for genuine
/// internal failures (e.g. couldn't spawn nvcc).
async fn run_job(
    tx: &mpsc::Sender<Result<ComputeResponse, Status>>,
    req: ComputeRequest,
    working_dir: &Path,
) -> anyhow::Result<()> {
    // 1. Create temporary workspace and reconstruct the uploaded directory tree
    fs::create_dir_all(working_dir)
        .await
        .context("Failed to create workspace")?;

    // Platform-agnostic output binary name
    let bin_name = if cfg!(windows) { "app.exe" } else { "app.out" };
    let is_multi_file = req.files.len() > 1;

    utils::prepare_workspace(working_dir, req.files)
        .await
        .context("Failed to prepare workspace")?;

    // 2. Compile with NVCC
    let compile_output = utils::build_nvcc_command(
        &req.entry_point_file,
        &req.compiler_flags,
        is_multi_file,
        bin_name,
    )
    .current_dir(working_dir)
    .output()
    .await
    .context("Failed to spawn nvcc")?;

    send_output(tx, u8_to_string(&compile_output.stdout), false).await;

    if !compile_output.status.success() {
        send_output(
            tx,
            format!(
                "❌ Compilation failed. Full error:\n{}",
                u8_to_string(&compile_output.stderr)
            ),
            true,
        )
        .await;
        return Ok(());
    }

    send_output(tx, "🚀 Compilation successful. Running binary...".into(), false).await;

    // 3. Execute the compiled binary with a hard timeout
    let exec_future = AsyncCommand::new(working_dir.join(bin_name))
        .current_dir(working_dir)
        .output();

    match timeout(Duration::from_secs(EXECUTION_TIMEOUT_SECS), exec_future).await {
        Ok(Ok(exec_output)) => {
            send_output(tx, u8_to_string(&exec_output.stdout), false).await;
            send_output(tx, u8_to_string(&exec_output.stderr), true).await;
        }
        Ok(Err(e)) => send_output(tx, format!("❌ Execution failed: {}", e), true).await,
        Err(_) => send_output(tx, "⏱️ Execution timed out. Process killed.".into(), true).await,
    }

    Ok(())
}

pub struct HostExecutor;

#[tonic::async_trait]
impl CudaExecutor for HostExecutor {
    type ExecuteCodeStream = ReceiverStream<Result<ComputeResponse, Status>>;

    async fn get_gpu_status(
        &self,
        _request: Request<common::compute::Empty>,
    ) -> Result<Response<common::compute::GpuStatus>, Status> {
        if let Some((name, temp, used, total, load)) = utils::get_nvidia_status().await {
            Ok(Response::new(common::compute::GpuStatus {
                gpu_name: name,
                temperature_celsius: temp,
                memory_used_mb: used,
                memory_total_mb: total,
                load_percentage: load,
            }))
        } else {
            Err(Status::unavailable("Could not query NVIDIA SMI"))
        }
    }

    async fn execute_code(
        &self,
        request: Request<ComputeRequest>,
    ) -> Result<Response<Self::ExecuteCodeStream>, Status> {
        let req = request.into_inner();
        let (tx, rx) = mpsc::channel(100);

        tokio::spawn(async move {
            let job_id = uuid::Uuid::new_v4().to_string();
            let working_dir = Path::new("scratch").join(&job_id);

            if let Err(e) = run_job(&tx, req, &working_dir).await {
                send_output(&tx, format!("❌ Internal error: {}", e), true).await;
            }

            // Cleanup the working directory regardless of job outcome
            let _ = fs::remove_dir_all(&working_dir).await;
            println!("🧹 Cleaned up job {}", job_id);
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // --- Pre-flight Check ---
    if cfg!(windows) {
        if let Some(path) = utils::find_msvc_x64_bin() {
            println!("✅ Environment Check: MSVC x64 detected at {:?}", path);
        } else {
            eprintln!("❌ Environment Error: MSVC x64 compiler (cl.exe) not found.");
            eprintln!("Please ensure 'Desktop development with C++' is installed in Visual Studio.");
            std::process::exit(1);
        }
    }

    // 1. Load .env file
    let _ = dotenvy::dotenv();

    // 2. Parse CLI arguments (Clap handles the env fallback automatically now)
    let args = HostArgs::parse();

    let addr = "0.0.0.0:50051".parse()?;
    let executor = HostExecutor;

    fs::create_dir_all("scratch").await?;

    println!("🦀 Ferris-Compute-Cuda Host listening on {} (Authenticated)", addr);

    // 3. Pass token into interceptor closure. We clone 'args.token' into closure
    // Use 'move' so the closure takes its own copy of 'args.token'
    let service = CudaExecutorServer::with_interceptor(executor, move |req| {
        let token_to_verify = args.token.clone();
        check_auth(req, token_to_verify)
    });
    Server::builder().add_service(service).serve(addr).await?;

    Ok(())
}

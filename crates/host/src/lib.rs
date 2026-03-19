//! Host library for remote CUDA execution server.
//! Exposes HostArgs, check_auth, HostExecutor, and utils for testing.

mod utils;

pub use utils::{build_nvcc_command, find_msvc_x64_bin, get_nvidia_status, prepare_workspace, u8_to_string};

use anyhow::Context;
use clap::Parser;
use common::compute::cuda_executor_server::{CudaExecutor, CudaExecutorServer};
use common::compute::{ComputeRequest, ComputeResponse};
use mdns_sd::{ServiceDaemon, ServiceInfo};
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
pub struct HostArgs {
    // Auth token. Priority: CLI Flag > Env Var > .env file
    #[arg(short, long, env = "FERRIS_AUTH_TOKEN")]
    pub token: String,
}

/// The Interceptor: This function runs BEFORE the service logic.
/// It checks if the 'x-ferris-token' matches our server's secret.
pub fn check_auth(req: Request<()>, expected_token: &str) -> Result<Request<()>, Status> {
    match req.metadata().get("x-ferris-token") {
        Some(token) if token == expected_token => Ok(req),
        _ => Err(Status::unauthenticated("Invalid or missing auth token")),
    }
}

async fn send_output(tx: &mpsc::Sender<Result<ComputeResponse, Status>>, output: String, is_error: bool) {
    if !output.is_empty() {
        let _ = tx.send(Ok(ComputeResponse { output, is_error })).await;
    }
}

/// Drives the full compile-and-run pipeline for a single job.
async fn run_job(
    tx: &mpsc::Sender<Result<ComputeResponse, Status>>,
    req: ComputeRequest,
    working_dir: &Path,
) -> anyhow::Result<()> {
    fs::create_dir_all(working_dir)
        .await
        .context("Failed to create workspace")?;

    let bin_name = if cfg!(windows) { "app.exe" } else { "app.out" };
    let is_multi_file = req.files.len() > 1;

    prepare_workspace(working_dir, req.files)
        .await
        .context("Failed to prepare workspace")?;

    let compile_output = build_nvcc_command(&req.entry_point_file, &req.compiler_flags, is_multi_file, bin_name)
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
        if let Some((name, temp, used, total, load)) = get_nvidia_status().await {
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

            let _ = fs::remove_dir_all(&working_dir).await;
            println!("🧹 Cleaned up job {}", job_id);
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

const MDNS_SERVICE_TYPE: &str = "_ferris-compute._tcp.local.";
const HOST_PORT: u16 = 50051;

fn register_mdns(port: u16) -> Result<ServiceDaemon, Box<dyn std::error::Error>> {
    let mdns = ServiceDaemon::new()?;
    let hostname = hostname::get()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let instance_name = format!("ferris-compute-{}", &hostname);
    let properties = [("version", env!("CARGO_PKG_VERSION"))];
    let service = ServiceInfo::new(
        MDNS_SERVICE_TYPE,
        &instance_name,
        &format!("{}.", hostname),
        "",
        port,
        &properties[..],
    )?;
    mdns.register(service)?;
    Ok(mdns)
}

/// Starts the host server. Used by main; exposed for testing.
pub async fn run_server(args: HostArgs) -> Result<(), Box<dyn std::error::Error>> {
    if cfg!(windows) {
        if let Some(path) = find_msvc_x64_bin() {
            println!("✅ Environment Check: MSVC x64 detected at {:?}", path);
        } else {
            eprintln!("❌ Environment Error: MSVC x64 compiler (cl.exe) not found.");
            eprintln!("Please ensure 'Desktop development with C++' is installed in Visual Studio.");
            std::process::exit(1);
        }
    }

    let addr = format!("0.0.0.0:{}", HOST_PORT).parse()?;
    let executor = HostExecutor;

    fs::create_dir_all("scratch").await?;

    match register_mdns(HOST_PORT) {
        Ok(_mdns) => println!("📡 mDNS: Advertising as {} on port {}", MDNS_SERVICE_TYPE, HOST_PORT),
        Err(e) => eprintln!("⚠️  mDNS registration failed (non-fatal): {}", e),
    }

    println!("🦀 Ferris-Compute-Cuda Host listening on {} (Authenticated)", addr);

    let token = args.token.clone();
    let service = CudaExecutorServer::with_interceptor(executor, move |req| {
        check_auth(req, &token)
    });
    Server::builder().add_service(service).serve(addr).await?;

    Ok(())
}

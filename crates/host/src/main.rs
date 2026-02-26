use common::compute::cuda_executor_server::{CudaExecutor, CudaExecutorServer};
use common::compute::{ComputeRequest, ComputeResponse};
use std::path::Path;
use tokio::fs;
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server, Request, Response, Status};

pub struct HostExecutor;

#[tonic::async_trait]
impl CudaExecutor for HostExecutor {
    type ExecuteCodeStream = ReceiverStream<Result<ComputeResponse, Status>>;

    async fn execute_code(
        &self,
        request: Request<ComputeRequest>,
    ) -> Result<Response<Self::ExecuteCodeStream>, Status> {
        let req = request.into_inner();
        let (tx, rx) = mpsc::channel(100);

        tokio::spawn(async move {
            let job_id = uuid::Uuid::new_v4().to_string();
            let working_dir = Path::new("scratch").join(&job_id);

            // 1. Create temporary workspace
            if let Err(e) = fs::create_dir_all(&working_dir).await {
                let _ = tx.send(Err(Status::internal(format!("Failed to create workspace: {}", e)))).await;
                return;
            }

            let file_path = working_dir.join(&req.file_name);
            // Platform agnostic binary extension
            let bin_name = if cfg!(windows) { "app.exe" } else { "app.out" };
            let bin_path = working_dir.join(bin_name);

            // 2. Write source code
            let _ = fs::write(&file_path, &req.source_code).await;

            // 3. Compile with NVCC
            let compile_status = Command::new("nvcc")
                .arg(&file_path)
                .args(&req.compiler_flags)
                .arg("-o")
                .arg(&bin_path)
                .current_dir(&working_dir)
                .status()
                .await;

            match compile_status {
                Ok(s) if s.success() => {
                    let _ = tx.send(Ok(ComputeResponse { 
                        output: "🚀 Compilation successful. Running...".into(), 
                        is_error: false 
                    })).await;
                    
                    // 4. Execute the binary
                    let output = Command::new(&bin_path)
                        .current_dir(&working_dir)
                        .output()
                        .await;

                    if let Ok(out) = output {
                        let stdout = String::from_utf8_lossy(&out.stdout);
                        let stderr = String::from_utf8_lossy(&out.stderr);
                        
                        if !stdout.is_empty() {
                            let _ = tx.send(Ok(ComputeResponse { 
                                output: stdout.to_string(), 
                                is_error: false 
                            })).await;
                        }
                        if !stderr.is_empty() {
                            let _ = tx.send(Ok(ComputeResponse { 
                                output: stderr.to_string(), 
                                is_error: true 
                            })).await;
                        }
                    }
                }
                _ => {
                    let _ = tx.send(Ok(ComputeResponse { 
                        output: "❌ Compilation failed.".into(), 
                        is_error: true 
                    })).await;
                }
            }

            // 5. Cleanup: Delete the entire job directory
            let _ = fs::remove_dir_all(&working_dir).await;
            println!("🧹 Cleaned up job {}", job_id);
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let executor = HostExecutor;

    // Ensure the base scratch directory exists before we start accepting jobs
    fs::create_dir_all("scratch").await?;

    println!("🦀 Ferris-Compute-Cuda Host listening on {}", addr);

    // Start the gRPC server
    Server::builder()
        .add_service(CudaExecutorServer::new(executor))
        .serve(addr)
        .await?;

    Ok(())
}
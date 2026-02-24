/// This is a simplified MVP version. It handles the "Happy Path" of receiving code and calling the compiler.
use common::compute::cuda_executor_server::{CudaExecutor, CudaExecutorServer};
use common::compute::{ComputeRequest, ComputeResponse};
use tonic::{transport::Server, Request, Response, Status};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use std::process::Stdio;
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};

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
            // 1. Create a workspace (In a real project, use a temp dir)
            let job_id = uuid::Uuid::new_v4().to_string();
            let file_path = format!("{}.cu", job_id);
            let bin_path = format!("{}.out", job_id);

            std::fs::write(&file_path, &req.source_code).unwrap();

            // 2. Prepare NVCC command
            let mut child = Command::new("nvcc")
                .arg(&file_path)
                .arg("-o")
                .arg(&bin_path)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("failed to spawn nvcc");

            // 3. Stream NVCC output back to Client
            let stderr = child.stderr.take().unwrap();
            let mut reader = BufReader::new(stderr).lines();

            while let Ok(Some(line)) = reader.next_line().await {
                let _ = tx.send(Ok(ComputeResponse {
                    output: line,
                    is_error: true,
                })).await;
            }

            // 4. (Simplified) Execute the binary and cleanup here...
            let _ = child.wait().await;
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let executor = HostExecutor;

    println!("Ferris-Compute-Cuda Host listening on {}", addr);

    Server::builder()
        .add_service(CudaExecutorServer::new(executor))
        .serve(addr)
        .await?;

    Ok(())
}

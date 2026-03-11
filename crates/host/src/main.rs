/// Binary entry point. Delegates to the host library.
use clap::Parser;
use host::{run_server, HostArgs};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();
    let args = HostArgs::parse();
    run_server(args).await
}

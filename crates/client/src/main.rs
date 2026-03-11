/// Binary entry point. Delegates to the client library.
use clap::Parser;
use client::{handle_run, handle_status, Args};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///////// PRE-FLIGHT CHECKS /////////
    // 1. Load .env file into process env variable
    let _ = dotenvy::dotenv();

    // 2. Parse CLI arguments (Clap handles the env fallback automatically now)
    let args = Args::parse();

    match args {
        Args::Run {
            inputs,
            server,
            flags,
            token,
        } => handle_run(inputs, server, flags, token).await?,
        Args::Status { server, token } => handle_status(server, token).await?,
    }

    Ok(())
}

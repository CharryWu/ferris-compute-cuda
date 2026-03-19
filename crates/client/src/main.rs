/// Binary entry point. Delegates to the client library.
use clap::Parser;
use client::{config, handle_run, handle_status, interactive, resolve_server, Args};

const DEFAULT_SERVER: &str = "http://[::1]:50051";

fn resolve_or_prompt(cli_server: Option<String>) -> Result<String, Box<dyn std::error::Error>> {
    if let Some(server) = resolve_server(cli_server) {
        return Ok(server);
    }

    if interactive::is_interactive() {
        let history = config::load_history();
        interactive::prompt_server_selection(&history)
    } else {
        eprintln!(
            "Warning: No --server specified and stdin is not a TTY. Falling back to {}",
            DEFAULT_SERVER
        );
        Ok(DEFAULT_SERVER.to_string())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();
    let args = Args::parse();

    match args {
        Args::Run {
            inputs,
            server,
            flags,
            token,
        } => {
            let server = resolve_or_prompt(server)?;
            handle_run(inputs, &server, flags, token).await?
        }
        Args::Status { server, token } => {
            let server = resolve_or_prompt(server)?;
            handle_status(&server, token).await?
        }
        Args::Discover => {
            println!("🔍 Scanning local network for ferris-compute hosts (3s)...");
            let hosts = client::discovery::discover_hosts(3);
            if hosts.is_empty() {
                println!("No hosts found on the local network.");
            } else {
                println!("\n--- Discovered Hosts ---");
                for host in &hosts {
                    println!("  {} ({})", host.url(), host.hostname);
                }
                println!("------------------------\n");
            }
        }
    }

    Ok(())
}

use crate::config::{self, HistoryEntry};
use crate::discovery;
use chrono::Utc;
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};

const NEW_ADDRESS_LABEL: &str = "[ Enter a new address ]";
const DISCOVERY_TIMEOUT_SECS: u64 = 3;

pub fn validate_server_address(addr: &str) -> Result<(), String> {
    if !addr.starts_with("http://") && !addr.starts_with("https://") {
        return Err("Address must start with http:// or https://".into());
    }

    let without_scheme = addr
        .strip_prefix("https://")
        .or_else(|| addr.strip_prefix("http://"))
        .unwrap_or(addr);

    if !without_scheme.contains(':') {
        return Err("Address must include a port (e.g., http://10.0.0.1:50051)".into());
    }

    let parts: Vec<&str> = without_scheme.rsplitn(2, ':').collect();
    if parts.len() != 2 {
        return Err("Invalid address format".into());
    }

    let port_str = parts[0];
    port_str
        .parse::<u16>()
        .map_err(|_| format!("Invalid port: {}", port_str))?;

    let host = parts[1];
    if host.is_empty() {
        return Err("Host cannot be empty".into());
    }

    Ok(())
}

fn format_relative_time(entry: &HistoryEntry) -> String {
    let duration = Utc::now().signed_duration_since(entry.last_used);
    let secs = duration.num_seconds();
    if secs < 60 {
        "just now".to_string()
    } else if secs < 3600 {
        format!("{} min ago", secs / 60)
    } else if secs < 86400 {
        format!("{} hours ago", secs / 3600)
    } else {
        format!("{} days ago", secs / 86400)
    }
}

/// Presents an interactive server selection prompt with LAN discovery and history.
/// Returns the chosen server address or an error if cancelled / non-interactive.
pub fn prompt_server_selection(history: &[HistoryEntry]) -> Result<String, Box<dyn std::error::Error>> {
    let theme = ColorfulTheme::default();

    println!("🔍 Scanning local network for hosts ({DISCOVERY_TIMEOUT_SECS}s)...");
    let discovered = discovery::discover_hosts(DISCOVERY_TIMEOUT_SECS);

    if discovered.is_empty() && history.is_empty() {
        println!("No hosts discovered and no previous connections found.");
        return prompt_new_address(&theme);
    }

    let mut items: Vec<String> = Vec::new();
    let mut urls: Vec<String> = Vec::new();

    if !discovered.is_empty() {
        for host in &discovered {
            items.push(format!("{}  ({})", host.url(), host.hostname));
            urls.push(host.url());
        }
    }

    for entry in history {
        if urls.contains(&entry.server) {
            continue;
        }
        let time = format_relative_time(entry);
        let label = match &entry.label {
            Some(l) => format!("{}  ({}, {})", entry.server, l, time),
            None => format!("{}  ({})", entry.server, time),
        };
        items.push(label);
        urls.push(entry.server.clone());
    }

    items.push(NEW_ADDRESS_LABEL.to_string());

    let selection = Select::with_theme(&theme)
        .with_prompt("No --server provided. Select a connection")
        .items(&items)
        .default(0)
        .interact()?;

    if selection == items.len() - 1 {
        return prompt_new_address(&theme);
    }

    Ok(urls[selection].clone())
}

fn prompt_new_address(theme: &ColorfulTheme) -> Result<String, Box<dyn std::error::Error>> {
    let addr: String = Input::with_theme(theme)
        .with_prompt("Enter server address (e.g., http://10.0.0.1:50051)")
        .validate_with(|input: &String| validate_server_address(input))
        .interact_text()?;

    if Confirm::with_theme(theme)
        .with_prompt(format!("Save {} as your default server?", addr))
        .default(true)
        .interact()?
    {
        config::save_default_server(&addr);
        println!("Saved to ~/.ferris-compute/config.toml");
    }

    Ok(addr)
}

pub fn is_interactive() -> bool {
    atty::is(atty::Stream::Stdin) && atty::is(atty::Stream::Stdout)
}

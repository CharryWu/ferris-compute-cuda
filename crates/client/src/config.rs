use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const MAX_HISTORY_ENTRIES: usize = 20;

#[derive(Deserialize)]
pub struct Config {
    pub default: Option<DefaultConfig>,
}

#[derive(Deserialize)]
pub struct DefaultConfig {
    pub server: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HistoryEntry {
    pub server: String,
    pub last_used: DateTime<Utc>,
    pub label: Option<String>,
}

pub fn config_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".ferris-compute"))
}

pub fn load_config() -> Option<Config> {
    let path = config_dir()?.join("config.toml");
    let content = fs::read_to_string(path).ok()?;
    toml::from_str(&content).ok()
}

pub fn load_history() -> Vec<HistoryEntry> {
    let Some(path) = config_dir().map(|d| d.join("history.json")) else {
        return vec![];
    };
    let Ok(content) = fs::read_to_string(path) else {
        return vec![];
    };
    serde_json::from_str(&content).unwrap_or_default()
}

pub fn save_history_entry(server: &str) {
    let Some(dir) = config_dir() else { return };
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("history.json");

    let mut history = load_history();

    if let Some(entry) = history.iter_mut().find(|e| e.server == server) {
        entry.last_used = Utc::now();
    } else {
        history.push(HistoryEntry {
            server: server.to_string(),
            last_used: Utc::now(),
            label: None,
        });
    }

    history.sort_by(|a, b| b.last_used.cmp(&a.last_used));
    history.truncate(MAX_HISTORY_ENTRIES);

    if let Ok(json) = serde_json::to_string_pretty(&history) {
        let _ = fs::write(path, json);
    }
}

pub fn save_default_server(server: &str) {
    let Some(dir) = config_dir() else { return };
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("config.toml");

    let content = format!(
        "[default]\nserver = \"{}\"\n",
        server.replace('\\', "\\\\").replace('"', "\\\"")
    );
    let _ = fs::write(path, content);
}

/// Resolves the server address from config file. Returns None if no config or no server set.
pub fn resolve_server_from_config() -> Option<String> {
    let config = load_config()?;
    config.default?.server
}

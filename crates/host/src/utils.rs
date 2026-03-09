use std::path::{Path, PathBuf};
use std::process::Command as SyncCommand;
use tokio::fs;
use anyhow::Context; // Adds the .context() method to Results
use std::collections::HashMap;

/// Dynamically locates the Microsoft Visual C++ (MSVC) x64 compiler toolchain.
///
/// This function is essential for Windows hosts because `nvcc` requires a host compiler
/// (`cl.exe`) to function. Instead of hardcoding versions (which change with Visual Studio
/// updates), this utility:
///
/// 1. Locates `vswhere.exe` (the standard VS installer metadata tool).
/// 2. Queries the latest installation containing the C++ Desktop development workload.
/// 3. Traverses the versioned MSVC toolset directory to find the highest version.
/// 4. Returns the path to the `Hostx64\x64` binary folder.
///
/// # Returns
/// * `Some(PathBuf)` - The absolute path to the directory containing `cl.exe`.
/// * `None` - If Visual Studio, the C++ workload, or the 64-bit toolset is missing.
///
/// # Platform
/// This function is intended for Windows. On Linux/macOS, it will typically fail
/// or return `None` as `vswhere` will not exist.
pub fn find_msvc_x64_bin() -> Option<PathBuf> {
    // Standard hidden location for vswhere.exe if not in system PATH
    let vswhere_path = r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe";
    let cmd_name = if Path::new(vswhere_path).exists() {
        vswhere_path
    } else {
        "vswhere"
    };

    // Execute vswhere to find the latest VS installation with C++ tools
    let output = SyncCommand::new(cmd_name)
        .args([
            "-latest",
            "-products",
            "*",
            "-requires",
            "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
            "-format",
            "json",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    // Parse the JSON output to get the installation root
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    let install_path_str = json.as_array()?.get(0)?["installationPath"].as_str()?;

    // Build path to: VC\Tools\MSVC
    let mut path = PathBuf::from(install_path_str);
    path.push("VC");
    path.push("Tools");
    path.push("MSVC");

    if !path.exists() {
        return None;
    }

    // Find all versioned folders (e.g., 14.44.35207) and pick the latest
    let mut entries: Vec<_> = std::fs::read_dir(&path)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();

    entries.sort();
    let latest_version = entries.last()?;

    // Target the 64-bit host compiler for 64-bit GPU kernels
    let bin_path = latest_version.join("bin").join("Hostx64").join("x64");

    if bin_path.exists() {
        Some(bin_path)
    } else {
        None
    }
}

pub async fn get_nvidia_status() -> Option<(String, u32, u32, u32, u32)> {
    let output = SyncCommand::new("nvidia-smi")
        .args([
            "--query-gpu=name,temperature.gpu,memory.used,memory.total,utilization.gpu",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = stdout.split(',').map(|s| s.trim()).collect();

    if parts.len() >= 5 {
        Some((
            parts[0].to_string(),
            parts[1].parse().unwrap_or(0),
            parts[2].parse().unwrap_or(0),
            parts[3].parse().unwrap_or(0),
            parts[4].parse().unwrap_or(0),
        ))
    } else {
        None
    }
}

/// Reconstructs the uploaded files into the provided working directory
pub async fn prepare_workspace(
    working_dir: &Path, 
    files: HashMap<String, String>
) -> anyhow::Result<()> {
    for (name, content) in files {
        let file_path = working_dir.join(&name);
        
        // Handle nested directories
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)
                .await
                .context(format!("Failed to create subdirectory: {:?}", parent))?;
        }
        
        fs::write(&file_path, content)
            .await
            .context(format!("Failed to write file content to: {:?}", file_path))?;
    }
    Ok(())
}
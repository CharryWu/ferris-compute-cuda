use std::collections::HashMap;
use std::path::Path;

/// Recursively gathers valid CUDA/C++ files from a path (file or directory)
pub fn gather_files_recursive(
    base_dir: &Path,     // The fixed "root" of the sync (e.g., "project/")
    current_path: &Path, // The current file/folder being visited
    files_map: &mut HashMap<String, String>,
) -> Result<(), Box<dyn std::error::Error>> {
    const ALLOWED_EXTENSIONS: [&str; 6] = ["cu", "cuh", "ptx", "cubin", "h", "cpp"];

    if current_path.is_dir() {
        for entry in std::fs::read_dir(current_path)? {
            // Keep passing the original base_dir down so prefix stripping stays consistent
            gather_files_recursive(base_dir, &entry?.path(), files_map)?;
        }
    } else {
        let extension = current_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default()
            .to_lowercase();

        if ALLOWED_EXTENSIONS.contains(&extension.as_str()) {
            let content = std::fs::read_to_string(current_path)
                .map_err(|e| format!("Could not read file {}: {}", current_path.display(), e))?;

            // 1. Strip the base_dir to get the true relative path
            // 2. Normalize to forward slashes (/) for gRPC/Host compatibility
            let relative_path = current_path
                .strip_prefix(base_dir)?
                .to_string_lossy()
                .replace('\\', "/");

            files_map.insert(relative_path, content);
        }
    }
    Ok(())
}
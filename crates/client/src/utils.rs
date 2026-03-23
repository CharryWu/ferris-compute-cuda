use std::collections::HashMap;
use std::path::Path;
use std::fs::read_to_string;

const ALLOWED_EXTENSIONS: [&str; 7] = ["cu", "cuh", "ptx", "cubin", "h", "cpp", "hpp"];

fn is_match(pattern: &str, name: &str) -> bool {
    pattern == name || pattern.ends_with("*") && name.starts_with(&pattern[..pattern.len() - 1])
}

/// Recursively gathers valid CUDA/C++ files from a path (file or directory)
/// Respects the provided ignore_list for both folders and files.
pub fn gather_files_recursive(
    base_dir: &Path,                         // The fixed "root" of the sync (e.g., "project/")
    current_path: &Path,                     // The current file/folder being visited
    files_map: &mut HashMap<String, String>, // File name -> Content pairs, initially empty map
    ignore_list: &Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Get the local name (e.g., "math" or "kernel.cu")
    let name = current_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();

    // 2. Check against ignore list (exact match for names like "target" or ".git")
    // This handles both files and directories.
    if ignore_list.iter().any(|pattern| is_match(pattern, name)) {
        return Ok(()); // Early return if the current node is among ignore list
    }

    if current_path.is_dir() {
        // 3. DIR LOGIC: Recurse into its child nodes (includes both files and directories)
        for entry in std::fs::read_dir(current_path)? {
            // Keep passing the original base_dir down so prefix stripping stays consistent
            gather_files_recursive(base_dir, &entry?.path(), files_map, &ignore_list)?;
        }
    } else {
        // 4. FILE LOGIC: Check extension and sync.
        // At this point, the file at `current_path.file_name()` is NOT among ignore list
        // (see above early return) so we can proceed to sync it.
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

/// Reads the .ferrisignore from base directory and returns a list of patterns to ignore
pub fn read_ignore(base_dir: &Path) -> Vec<String> {
    let mut ignores = vec![
        ".git".to_string(),
        "target".to_string(),
        ".DS_Store".to_string(),
        ".env".to_string(),
        ".ferrisignore".to_string(),
    ];

    let ignore_file = base_dir.join(".ferrisignore");
    if let Ok(content) = read_to_string(ignore_file) {
        for line in content.lines() {
            let trimmed_line = line.trim();
            // Skip comments and empty lines
            if !trimmed_line.is_empty() && !trimmed_line.starts_with('#') {
                ignores.push(trimmed_line.to_string());
            }
        }
    }

    ignores
}

#[cfg(test)]
mod tests {
    use super::is_match;

    #[test]
    fn is_match_exact_name() {
        assert!(is_match("target", "target"));
        assert!(is_match(".git", ".git"));
        assert!(!is_match("target", "targets"));
        assert!(!is_match("target", "Target"));
    }

    #[test]
    fn is_match_prefix_wildcard() {
        assert!(is_match("build*", "build"));
        assert!(is_match("build*", "build_output"));
        assert!(is_match("*.log", "*.log")); // literal filename "*.log" in ignore file
        assert!(!is_match("build*", "rebuild"));
    }

    #[test]
    fn is_match_star_only_matches_anything() {
        assert!(is_match("*", "anything"));
        assert!(is_match("*", ""));
    }

    #[test]
    fn is_match_star_not_at_end_is_exact_only() {
        // Only trailing `*` triggers prefix semantics; otherwise full string equality.
        assert!(is_match("pre*fix", "pre*fix"));
        assert!(!is_match("pre*fix", "prefix"));
    }
}
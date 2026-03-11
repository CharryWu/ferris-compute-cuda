use client::{gather_files_recursive, read_ignore};
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

fn create_test_dir() -> TempDir {
    TempDir::new().expect("Failed to create temp dir")
}

#[test]
fn read_ignore_returns_defaults_when_no_ferrisignore() {
    let dir = create_test_dir();
    let ignores = read_ignore(dir.path());
    assert!(ignores.contains(&".git".to_string()));
    assert!(ignores.contains(&"target".to_string()));
    assert!(ignores.contains(&".DS_Store".to_string()));
    assert!(ignores.contains(&".env".to_string()));
    assert!(ignores.contains(&".ferrisignore".to_string()));
}

#[test]
fn read_ignore_adds_custom_patterns_from_file() {
    let dir = create_test_dir();
    fs::write(dir.path().join(".ferrisignore"), "build\n*.log\nnode_modules").unwrap();
    let ignores = read_ignore(dir.path());
    assert!(ignores.contains(&"build".to_string()));
    assert!(ignores.contains(&"*.log".to_string()));
    assert!(ignores.contains(&"node_modules".to_string()));
}

#[test]
fn read_ignore_skips_comments_and_empty_lines() {
    let dir = create_test_dir();
    let content = "# comment line\n\n  build  \n\n# another comment\nvendor\n";
    fs::write(dir.path().join(".ferrisignore"), content).unwrap();
    let ignores = read_ignore(dir.path());
    assert!(ignores.contains(&"build".to_string()));
    assert!(ignores.contains(&"vendor".to_string()));
    assert!(!ignores.iter().any(|s| s.starts_with('#')));
    assert!(!ignores.contains(&"".to_string()));
}

#[test]
fn gather_files_recursive_single_allowed_file() {
    let dir = create_test_dir();
    let base = dir.path();
    let file_path = base.join("kernel.cu");
    fs::write(&file_path, "// CUDA code").unwrap();

    let mut files = HashMap::new();
    let ignore_list = vec![];
    gather_files_recursive(base, &file_path, &mut files, &ignore_list).unwrap();

    assert_eq!(files.len(), 1);
    assert!(files.contains_key("kernel.cu"));
    assert_eq!(files.get("kernel.cu").unwrap(), "// CUDA code");
}

#[test]
fn gather_files_recursive_ignores_disallowed_extensions() {
    let dir = create_test_dir();
    let base = dir.path();
    fs::write(base.join("script.py"), "print('hi')").unwrap();
    fs::write(base.join("readme.md"), "# readme").unwrap();
    fs::write(base.join("kernel.cu"), "// cuda").unwrap();

    let mut files = HashMap::new();
    gather_files_recursive(base, base, &mut files, &vec![]).unwrap();

    assert_eq!(files.len(), 1);
    assert!(files.contains_key("kernel.cu"));
    assert!(!files.contains_key("script.py"));
    assert!(!files.contains_key("readme.md"));
}

#[test]
fn gather_files_recursive_respects_ignore_list() {
    let dir = create_test_dir();
    let base = dir.path();
    fs::create_dir(base.join("target")).unwrap();
    fs::write(base.join("target").join("build.cu"), "// ignored").unwrap();
    fs::write(base.join("kernel.cu"), "// included").unwrap();

    let mut files = HashMap::new();
    let ignore_list = vec!["target".to_string()];
    gather_files_recursive(base, base, &mut files, &ignore_list).unwrap();

    assert_eq!(files.len(), 1);
    assert!(files.contains_key("kernel.cu"));
    assert!(!files.contains_key("target/build.cu"));
}

#[test]
fn gather_files_recursive_nested_structure() {
    let dir = create_test_dir();
    let base = dir.path();
    fs::create_dir_all(base.join("src").join("kernels")).unwrap();
    fs::write(base.join("main.cu"), "// main").unwrap();
    fs::write(base.join("src").join("util.h"), "// util").unwrap();
    fs::write(base.join("src").join("kernels").join("math.cuh"), "// math").unwrap();

    let mut files = HashMap::new();
    gather_files_recursive(base, base, &mut files, &vec![]).unwrap();

    assert_eq!(files.len(), 3);
    assert!(files.contains_key("main.cu"));
    assert!(files.contains_key("src/util.h"));
    assert!(files.contains_key("src/kernels/math.cuh"));
}

#[test]
fn gather_files_recursive_all_allowed_extensions() {
    let dir = create_test_dir();
    let base = dir.path();
    for ext in ["cu", "cuh", "ptx", "cubin", "h", "cpp", "hpp"] {
        fs::write(base.join(format!("file.{}", ext)), "content").unwrap();
    }

    let mut files = HashMap::new();
    gather_files_recursive(base, base, &mut files, &vec![]).unwrap();

    assert_eq!(files.len(), 7);
    for ext in ["cu", "cuh", "ptx", "cubin", "h", "cpp", "hpp"] {
        assert!(files.contains_key(&format!("file.{}", ext)));
    }
}

#[test]
fn gather_files_recursive_case_insensitive_extension() {
    let dir = create_test_dir();
    let base = dir.path();
    fs::write(base.join("kernel.CU"), "// uppercase").unwrap();
    fs::write(base.join("header.HPP"), "// header").unwrap();

    let mut files = HashMap::new();
    gather_files_recursive(base, base, &mut files, &vec![]).unwrap();

    assert_eq!(files.len(), 2);
    assert!(files.contains_key("kernel.CU"));
    assert!(files.contains_key("header.HPP"));
}

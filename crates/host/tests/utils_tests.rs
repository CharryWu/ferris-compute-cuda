use host::{prepare_workspace, u8_to_string};
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

#[test]
fn u8_to_string_valid_utf8() {
    let bytes = b"hello world";
    assert_eq!(u8_to_string(bytes), "hello world");
}

#[test]
fn u8_to_string_invalid_utf8_replaced() {
    let bytes = b"hello\xff\xfe world";
    let s = u8_to_string(bytes);
    assert!(s.contains("hello"));
    assert!(s.contains("world"));
    assert!(!s.contains('\0'));
}

#[test]
fn u8_to_string_empty() {
    assert_eq!(u8_to_string(b""), "");
}

#[tokio::test]
async fn prepare_workspace_single_file() {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let mut files = HashMap::new();
    files.insert("main.cu".to_string(), "// CUDA code".to_string());

    prepare_workspace(dir.path(), files).await.unwrap();

    let content = fs::read_to_string(dir.path().join("main.cu")).unwrap();
    assert_eq!(content, "// CUDA code");
}

#[tokio::test]
async fn prepare_workspace_nested_dirs() {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let mut files = HashMap::new();
    files.insert("src/kernels/math.cuh".to_string(), "// math header".to_string());
    files.insert("main.cu".to_string(), "// main".to_string());

    prepare_workspace(dir.path(), files).await.unwrap();

    assert!(dir.path().join("src/kernels/math.cuh").exists());
    assert!(dir.path().join("main.cu").exists());
    assert_eq!(
        fs::read_to_string(dir.path().join("src/kernels/math.cuh")).unwrap(),
        "// math header"
    );
}

#[tokio::test]
async fn prepare_workspace_multiple_files() {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let mut files = HashMap::new();
    files.insert("a.cu".to_string(), "content a".to_string());
    files.insert("b.h".to_string(), "content b".to_string());
    files.insert("sub/c.cuh".to_string(), "content c".to_string());

    prepare_workspace(dir.path(), files).await.unwrap();

    assert_eq!(fs::read_to_string(dir.path().join("a.cu")).unwrap(), "content a");
    assert_eq!(fs::read_to_string(dir.path().join("b.h")).unwrap(), "content b");
    assert_eq!(fs::read_to_string(dir.path().join("sub/c.cuh")).unwrap(), "content c");
}

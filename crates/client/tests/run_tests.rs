use clap::Parser;
use client::{handle_run, Args};
use std::path::PathBuf;

#[test]
fn args_run_parses_inputs_and_token() {
    let args = Args::parse_from([
        "ferris-run",
        "run",
        "main.cu",
        "src/",
        "--token",
        "test-token-123",
    ]);
    match &args {
        Args::Run { inputs, token, .. } => {
            assert_eq!(inputs.len(), 2);
            assert_eq!(inputs[0], PathBuf::from("main.cu"));
            assert_eq!(inputs[1], PathBuf::from("src/"));
            assert_eq!(token, "test-token-123");
        }
        _ => panic!("Expected Run variant"),
    }
}

#[test]
fn args_run_default_server() {
    let args = Args::parse_from(["ferris-run", "run", "main.cu", "--token", "t"]);
    match &args {
        Args::Run { server, .. } => assert_eq!(server, "http://[::1]:50051"),
        _ => panic!("Expected Run variant"),
    }
}

#[test]
fn args_run_with_flags() {
    let args = Args::parse_from([
        "ferris-run",
        "run",
        "main.cu",
        "--token",
        "t",
        "--flags=-arch=sm_80",
        "--flags=-O3",
    ]);
    match &args {
        Args::Run { flags, .. } => {
            assert_eq!(flags.len(), 2);
            assert_eq!(flags[0], "-arch=sm_80");
            assert_eq!(flags[1], "-O3");
        }
        _ => panic!("Expected Run variant"),
    }
}

#[test]
fn args_status_parses() {
    let args = Args::parse_from([
        "ferris-run",
        "status",
        "--server",
        "http://localhost:50051",
        "--token",
        "status-token",
    ]);
    match &args {
        Args::Status { server, token } => {
            assert_eq!(server, "http://localhost:50051");
            assert_eq!(token, "status-token");
        }
        _ => panic!("Expected Status variant"),
    }
}

#[test]
fn args_status_default_server() {
    let args = Args::parse_from(["ferris-run", "status", "--token", "t"]);
    match &args {
        Args::Status { server, .. } => assert_eq!(server, "http://[::1]:50051"),
        _ => panic!("Expected Status variant"),
    }
}

#[tokio::test]
async fn handle_run_empty_inputs_returns_error() {
    let result = handle_run(vec![], "http://[::1]:50051".into(), vec![], "token".into()).await;
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().to_string(),
        "No input files provided."
    );
}

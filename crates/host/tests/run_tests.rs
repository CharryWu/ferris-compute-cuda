use clap::Parser;
use host::{build_nvcc_command, check_auth, HostArgs};
use tonic::Request;

#[test]
fn host_args_parses_token() {
    let args = HostArgs::parse_from(["host", "--token", "my-secret-token"]);
    assert_eq!(args.token, "my-secret-token");
}

#[test]
fn check_auth_valid_token() {
    let mut req = Request::new(());
    let value = "valid-token".parse::<tonic::metadata::MetadataValue<tonic::metadata::Ascii>>().unwrap();
    req.metadata_mut().insert("x-ferris-token", value);

    let result = check_auth(req, "valid-token");
    assert!(result.is_ok());
}

#[test]
fn check_auth_invalid_token() {
    let mut req = Request::new(());
    let value = "wrong-token".parse::<tonic::metadata::MetadataValue<tonic::metadata::Ascii>>().unwrap();
    req.metadata_mut().insert("x-ferris-token", value);

    let result = check_auth(req, "expected-token");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), tonic::Code::Unauthenticated);
}

#[test]
fn check_auth_missing_token() {
    let req = Request::new(());

    let result = check_auth(req, "expected-token");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), tonic::Code::Unauthenticated);
}

#[test]
#[cfg(not(target_os = "windows"))]
fn build_nvcc_command_single_file() {
    let cmd = build_nvcc_command("main.cu", &[], false, "app.out");
    let args: Vec<_> = cmd.as_std().get_args().map(|a| a.to_string_lossy().into_owned()).collect();

    assert_eq!(args[0], "main.cu");
    assert!(args.contains(&"-I.".to_string()));
    assert!(args.contains(&"-o".to_string()));
    assert!(args.iter().any(|a| a == "app.out"));
    assert!(!args.contains(&"-rdc=true".to_string()));
}

#[test]
#[cfg(target_os = "windows")]
fn build_nvcc_command_single_file_windows() {
    let cmd = build_nvcc_command("main.cu", &[], false, "app.out");
    let args: Vec<_> = cmd.as_std().get_args().map(|a| a.to_string_lossy().into_owned()).collect();

    // On Windows with MSVC, -ccbin and path precede entry_point
    assert!(args.contains(&"-ccbin".to_string()));
    assert!(args.contains(&"main.cu".to_string()));
    assert!(args.contains(&"-I.".to_string()));
    assert!(args.contains(&"-o".to_string()));
    assert!(args.iter().any(|a| a == "app.out"));
    assert!(!args.contains(&"-rdc=true".to_string()));
}

#[test]
fn build_nvcc_command_multi_file_injects_rdc() {
    let cmd = build_nvcc_command("main.cu", &["-arch=sm_80".to_string()], true, "app.out");
    let args: Vec<_> = cmd.as_std().get_args().map(|a| a.to_string_lossy().into_owned()).collect();

    assert!(args.contains(&"-rdc=true".to_string()));
    assert!(args.contains(&"-arch=sm_80".to_string()));
}

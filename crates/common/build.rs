/// Custom build script that extends the default build script provided by tonic-build.
/// Requires `protoc` command to be available. See .github/workflows/ci.yml for more details.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // This compiles the .proto file into Rust code.
    // By default, the generated code is placed in the 'OUT_DIR' 
    // (inside the /target folder), keeping your src/ directory clean.
    tonic_build::compile_protos("proto/compute.proto")?;
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // This compiles the .proto file into Rust code.
    // By default, the generated code is placed in the 'OUT_DIR' 
    // (inside the /target folder), keeping your src/ directory clean.
    tonic_build::compile_protos("proto/compute.proto")?;
    Ok(())
}
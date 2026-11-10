use std::{env, path::PathBuf};

// compile the protobuf
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);

    tonic_prost_build::configure()
        .file_descriptor_set_path(out_dir.join("telemetry_descriptor.bin"))
        .compile_protos(&["proto/telemetry.proto"], &["proto"])?;

    Ok(())
}

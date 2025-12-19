use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc_path = protoc_bin_vendored::protoc_bin_path()?;
    unsafe {
        std::env::set_var("PROTOC", protoc_path);
    }

    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);
    let descriptor_path = out_dir.join("descriptor.bin");

    tonic_prost_build::configure()
        .file_descriptor_set_path(&descriptor_path)
        .compile_protos(&["proto/snake_game.proto"], &["proto"])?;

    let descriptor_bytes = fs::read(&descriptor_path)?;

    pbjson_build::Builder::new()
        .register_descriptors(&descriptor_bytes)?
        .build(&["."])?;
    Ok(())
}

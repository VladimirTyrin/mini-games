use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_root = PathBuf::from("../../proto");
    let package_dir = proto_root.join("minigames/v2");
    let mut files = Vec::new();
    for entry in std::fs::read_dir(&package_dir)? {
        let path = entry?.path();
        if path.extension().is_some_and(|ext| ext == "proto") {
            println!("cargo:rerun-if-changed={}", path.display());
            files.push(path);
        }
    }
    files.sort();

    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);
    let descriptor_path = out_dir.join("descriptor.bin");
    let mut config = prost_build::Config::new();
    config.protoc_executable(protoc);
    config.file_descriptor_set_path(&descriptor_path);
    config.compile_protos(&files, &[proto_root])?;

    if std::env::var_os("CARGO_FEATURE_JSON").is_some() {
        let descriptors = std::fs::read(&descriptor_path)?;
        pbjson_build::Builder::new()
            .register_descriptors(&descriptors)?
            .build(&[".minigames.v2"])?;
    }
    Ok(())
}

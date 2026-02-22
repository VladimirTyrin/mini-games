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
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .compile_protos(
            &[
                "../proto/game_service.proto",
                "../proto/replay.proto",
                "../proto/games/snake.proto",
                "../proto/games/tictactoe.proto",
                "../proto/games/numbers_match.proto",
                "../proto/games/stack_attack.proto",
                "../proto/games/puzzle2048.proto",
            ],
            &["../proto"],
        )?;

    Ok(())
}

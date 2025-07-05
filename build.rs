fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::env::var("OUT_DIR")?;
    let descriptor_path = std::path::Path::new(&out_dir).join("descriptor.bin");

    tonic_build::configure()
        .file_descriptor_set_path(&descriptor_path)
        .compile(&["proto/cluster.proto"], &["proto"])?;

    Ok(())
}
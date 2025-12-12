fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Compiling protos...");
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(
            &["../atlas-common/proto/atlas.proto"], // list of protos to compile
            &["../atlas-common/proto"], // path to search for protos
        )?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "grpc")]
    tonic_build::compile_protos("../atlas-ledger/proto/ledger.proto")?;
    Ok(())
}

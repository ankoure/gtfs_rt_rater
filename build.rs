fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc_path = protoc_bin_vendored::protoc_bin_path()?;

    let mut config = prost_build::Config::new();
    config.protoc_path(protoc_path);

    config.compile_protos(
        &["proto/gtfs-realtime.proto"], // input proto
        &["proto/"],                    // proto include path
    )?;
    Ok(())
}

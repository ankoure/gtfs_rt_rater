fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Use vendored protoc
    let protoc_path = protoc_bin_vendored::protoc_bin_path()?;
    std::env::set_var("PROTOC", protoc_path);
    prost_build::compile_protos(
        &["proto/gtfs-realtime.proto"], // input proto
        &["proto/"],                    // proto include path
    )?;
    Ok(())
}

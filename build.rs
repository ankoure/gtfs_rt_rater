fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=src/gtfs_rt.proto");

    let protoc_path = protoc_bin_vendored::protoc_bin_path()?;

    unsafe {
        std::env::set_var("PROTOC", protoc_path);
    }

    prost_build::compile_protos(&["proto/gtfs-realtime.proto"], &["proto/"])?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protos = [
        "proto/user/v1/user.proto",
        "proto/warehouse/v1/warehouse.proto",
    ];

    // Scope reruns to the .proto files only; the build script writes into src/,
    // so watching everything would loop forever.
    for proto in &protos {
        println!("cargo:rerun-if-changed={proto}");
    }

    // protox is a pure-Rust protobuf compiler: no protoc binary needed, and it
    // bundles the well-known types so google/protobuf/*.proto imports resolve.
    let file_descriptors = protox::compile(protos, ["proto"])?;

    tonic_prost_build::configure()
        .build_client(true)
        .build_server(true)
        .out_dir("src/generated")
        .compile_fds(file_descriptors)?;

    Ok(())
}

fn main() {
    // Compile the client-facing gRPC interface (proto/kvs.proto) into
    // `$OUT_DIR/kvs.rs`, which `kvs::sidecar::pb` pulls in via
    // `tonic::include_proto!`. We point the code generator at a vendored
    // `protoc` binary (via `Config::protoc_executable`, not the `PROTOC`
    // environment variable) so the build needs neither a system protobuf
    // compiler nor a process-global env mutation.
    let protoc = protoc_bin_vendored::protoc_bin_path().expect("vendored protoc binary");
    let mut config = tonic_build::Config::new();
    config.protoc_executable(protoc);
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos_with_config(config, &["proto/kvs.proto"], &["proto"])
        .expect("compile proto/kvs.proto");

    // Generate the stageleft `__staged` module / macro entrypoints. Must run in
    // the same build script (a package has only one).
    stageleft_tool::gen_final!();
}

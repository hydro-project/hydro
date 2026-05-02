fn main() {
    std::env::set_var("PROTOC", protoc_bin_vendored::protoc_bin_path().unwrap());

    // Inter-node codec — prost only (no service needed).
    prost_build::compile_protos(&["proto/kvs_internal.proto"], &["proto/"]).unwrap();

    // Public gRPC service — tonic-build emits both server and client stubs;
    // the client is used by the docker_e2e_test to exercise the gRPC ingress.
    tonic_build::configure()
        .compile_protos(&["proto/kvs.proto"], &["proto/"])
        .unwrap();

    stageleft_tool::gen_final!();
}

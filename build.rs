fn main() {
    let protoc =
        protoc_bin_vendored::protoc_bin_path().expect("failed to locate bundled protoc binary");
    std::env::set_var("PROTOC", protoc);

    tonic_prost_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(&["proto/arpc/v2/service.proto"], &["proto"])
        .expect("failed to compile aRPC v2 protobuf definitions");
}

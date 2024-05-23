fn main() {
    protobuf_codegen::Codegen::new()
        .include("src")
        .inputs(["src/segment.proto", "src/triangle.proto"])
        .cargo_out_dir("rust_protobuf_protos")
        .run_from_script();

    prost_build::Config::new()
        .compile_protos(&["src/segment.proto", "src/triangle.proto"], &["src"])
        .unwrap();
}

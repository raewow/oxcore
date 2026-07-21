//! Compiles the BGS RPC protobuf definitions with prost.

fn main() {
    println!("cargo:rerun-if-changed=proto/bgs.proto");
    prost_build::compile_protos(&["proto/bgs.proto"], &["proto"])
        .expect("failed to compile bgs.proto");
}

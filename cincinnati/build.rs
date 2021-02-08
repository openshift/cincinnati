#[cfg(feature = "codegen-protoc")]
extern crate protoc_rust;

fn main() {
    #[cfg(feature = "codegen-protoc")]
    codegen_protoc();
}

#[cfg(feature = "codegen-protoc")]
fn codegen_protoc() {
    const SCHEMA_FILENAME: &str = "src/plugins/interface.proto";

    println!("Generating code from {}", SCHEMA_FILENAME);
    protoc_rust::Codegen::new()
        .out_dir("src/plugins/")
        .inputs(&[SCHEMA_FILENAME])
        .customize(Default::default())
        .run()
        .expect("protoc");
}

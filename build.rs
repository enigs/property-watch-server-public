fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::compile_protos("proto/propertywatch/v1/hello.proto")?;
    println!("cargo:rerun-if-changed=proto/propertywatch/v1/hello.proto");
    Ok(())
}

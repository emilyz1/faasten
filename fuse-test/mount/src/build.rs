use std::io::Result;

fn main() -> Result<()> {
    prost_build::compile_protos(&["protobuf/syscalls.proto", "protobuf/messages.proto"], &["protobuf/"])?;
    Ok(())
}
use std::io::Result;

fn main() -> Result<()> {
    prost_build::compile_protos(&["src/protobuf/syscalls.proto", "src/protobuf/messages.proto"], &["src/"])?;
    Ok(())
}
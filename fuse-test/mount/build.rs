use std::io::Result;

fn main() -> Result<()> {
    panic!("sjiewog");
    prost_build::compile_protos(&["src/syscalls.proto"], &["src/"])?;
    Ok(())
}
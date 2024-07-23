use std::env;
use std::fs;
use std::io::Result;

fn main() -> Result<()> {
    prost_build::compile_protos(&["src/syscalls.proto"], &["src/"])?;
    Ok(())
}
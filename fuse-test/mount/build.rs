use std::env;
use std::fs;
use std::io::Result;
use std::path::Path;

fn main() -> Result<()> {
    // Retrieve the OUT_DIR environment variable
    let out_dir = env::var("OUT_DIR").unwrap();
    println!("OUT_DIR: {}", out_dir);

    // List the contents of the OUT_DIR
    let paths = fs::read_dir(&out_dir).unwrap();
    println!("Contents of OUT_DIR:");
    for path in paths {
        println!("{}", path.unwrap().path().display());
    }
    prost_build::compile_protos(&["src/syscalls.proto", "src/protobuf/messages.proto"], &["src/"])?;
    Ok(())
}
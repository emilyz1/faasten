extern crate fuse;

use std::env;
use fuse::Filesystem;

struct JsonFilesystem;

impl Filesystem for JsonFilesystem {
}

fn main() {
    let mountpoint = match env::args().nth(1) {
        Some(path) => path,
        None => {
            println!("Usage: {} <MOUNTPOINT>", env::args().nth(0).unwrap());
            return;
        }
    };
    fuse::mount(JsonFilesystem, &mountpoint, &[]);
}
extern crate fuse;
extern crate time;
extern crate libc;
extern crate rustc_serialize;

use std::collections::BTreeMap;
use std::env;
use std::ffi::OsStr;
use libc::{ENOENT, EEXIST};
use time::Timespec;
use fuse::{FileAttr, FileType, Filesystem, Request, ReplyAttr, ReplyData, ReplyEntry, ReplyDirectory, ReplyCreate, ReplyEmpty, ReplyWrite};
use rustc_serialize::json;

struct JsonFilesystem {
    tree: json::Object,
    attrs: BTreeMap<u64, FileAttr>,
    inodes: BTreeMap<String, u64>,
    next_inode: u64,
}

impl JsonFilesystem {
    fn new(tree: &json::Object) -> JsonFilesystem {
        let mut attrs = BTreeMap::new();
        let mut inodes = BTreeMap::new();
        let ts = time::now().to_timespec();
        let attr = FileAttr {
            ino: 1,
            size: 0,
            blocks: 0,
            atime: ts,
            mtime: ts,
            ctime: ts,
            crtime: ts,
            kind: FileType::Directory,
            perm: 0o755,
            nlink: 2,
            uid: 0,
            gid: 0,
            rdev: 0,
            flags: 0,
        };
        attrs.insert(1, attr);
        inodes.insert("/".to_string(), 1);
        let mut fs = JsonFilesystem {
            tree: tree.clone(),
            attrs,
            inodes,
            next_inode: 2,
        };

        // Initialize inodes and attrs from the JSON tree
        fs.initialize_inodes_and_attrs();
        fs
    }

    fn initialize_inodes_and_attrs(&mut self) {
        let ts = time::now().to_timespec();
        for (key, value) in self.tree.iter() {
            let attr = FileAttr {
                ino: self.next_inode,
                size: value.pretty().to_string().len() as u64,
                blocks: 0,
                atime: ts,
                mtime: ts,
                ctime: ts,
                crtime: ts,
                kind: FileType::RegularFile,
                perm: 0o644,
                nlink: 1,
                uid: 0,
                gid: 0,
                rdev: 0,
                flags: 0,
            };
            self.attrs.insert(self.next_inode, attr);
            self.inodes.insert(key.clone(), self.next_inode);
            self.next_inode += 1;
        }
    }
}

impl Filesystem for JsonFilesystem {
    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        println!("getattr(ino={})", ino);
        match self.attrs.get(&ino) {
            Some(attr) => {
                let ttl = Timespec::new(1, 0);
                reply.attr(&ttl, attr);
            }
            None => reply.error(ENOENT),
        };
    }

    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        println!("lookup(parent={}, name={})", parent, name.to_str().unwrap());
        let inode = match self.inodes.get(name.to_str().unwrap()) {
            Some(inode) => inode,
            None => {
                reply.error(ENOENT);
                return;
            }
        };
        match self.attrs.get(inode) {
            Some(attr) => {
                let ttl = Timespec::new(1, 0);
                reply.entry(&ttl, attr, 0);
            }
            None => reply.error(ENOENT),
        };
    }

    fn read(&mut self,
            _req: &Request,
            ino: u64,
            fh: u64,
            offset: i64,
            size: u32,
            reply: ReplyData,) {
        println!(
            "read(ino={}, fh={}, offset={}, size={})",
            ino,
            fh,
            offset,
            size
        );
        for (key, &inode) in &self.inodes {
            if inode == ino {
                let value = &self.tree[key];
                reply.data(value.pretty().to_string().as_bytes());
                return;
            }
        }
        reply.error(ENOENT);
    }

    fn readdir(&mut self,
                _req: &Request,
                ino: u64,
                fh: u64,
                offset: i64,
                mut reply: ReplyDirectory,) {
        println!("readdir(ino={}, fh={}, offset={})", ino, fh, offset);
        if ino == 1 {
            if offset == 0 {
                reply.add(1, 0, FileType::Directory, ".");
                reply.add(1, 1, FileType::Directory, "..");
                for (key, &inode) in &self.inodes {
                    if inode == 1 {
                        continue;
                    }
                    let offset = inode as i64; // hack
                    println!("\tkey={}, inode={}, offset={}", key, inode, offset);
                    reply.add(inode, offset, FileType::RegularFile, key);
                }
            }
            reply.ok();
        } else {
            reply.error(ENOENT);
        }
    }

    fn mkdir(&mut self, _req: &Request, parent: u64, name: &OsStr, mode: u32, reply: ReplyEntry) {
        println!("mkdir(parent={}, name={}, mode={})", parent, name.to_str().unwrap(), mode);
        let name_str = name.to_str().unwrap();
        if self.inodes.contains_key(name_str) {
            reply.error(EEXIST);
            return;
        }
        let ts = time::now().to_timespec();
        let ino = self.next_inode;
        self.next_inode += 1;
        let attr = FileAttr {
            ino,
            size: 0,
            blocks: 0,
            atime: ts,
            mtime: ts,
            ctime: ts,
            crtime: ts,
            kind: FileType::Directory,
            perm: mode as u16,
            nlink: 2,
            uid: 0,
            gid: 0,
            rdev: 0,
            flags: 0,
        };
        self.attrs.insert(ino, attr);
        self.inodes.insert(name_str.to_string(), ino);
        let ttl = Timespec::new(1, 0);
        reply.entry(&ttl, &attr, 0);
    }

    fn write(&mut self, _req: &Request, ino: u64, fh: u64, offset: i64, data: &[u8], _flags: u32, reply: ReplyWrite) {
        println!("write(ino={}, fh={}, offset={}, size={})", ino, fh, offset, data.len());
        for (key, &inode) in &self.inodes {
            if inode == ino {
                let content = match self.tree.get_mut(key) {
                    Some(content) => content,
                    None => {
                        reply.error(ENOENT);
                        return;
                    }
                };
                let mut content_str = content.pretty().to_string();
                let new_data = std::str::from_utf8(data).unwrap();
                content_str.insert_str(offset as usize, new_data);
                self.tree.insert(key.clone(), json::Json::String(content_str.clone())); // Clone here
                let attr = self.attrs.get_mut(&ino).unwrap();
                attr.size = content_str.len() as u64; // Use the cloned content_str
                reply.written(data.len() as u32);
                return;
            }
        }
        reply.error(ENOENT);
    }
    
}

fn main() {
    let data = json::Json::from_str("{\"foo\": \"bar\", \"answer\": 42}").unwrap();
    let tree = data.as_object().unwrap();
    let fs = JsonFilesystem::new(tree);
    let mountpoint = match env::args().nth(1) {
        Some(path) => path,
        None => {
            println!("Usage: {} <MOUNTPOINT>", env::args().nth(0).unwrap());
            return;
        }
    };
    fuse::mount(fs, &mountpoint, &[]).expect("Couldn't mount filesystem");
}

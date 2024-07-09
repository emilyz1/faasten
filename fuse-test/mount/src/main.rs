include!(concat!(env!("OUT_DIR"), "/snapfaas.syscalls.rs"));
// include!(concat!(env!("OUT_DIR"), "/sched.messages.rs"));

extern crate vsock;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use protobuf::Message;
use std::io::{Read, Write};
use std::net::TcpStream;
// use vsock::VsockStream;
use clap::{crate_version, Arg, ArgAction, Command};
use fuser::{
    FileAttr, FileType, Filesystem, MountOption, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry
};
use libc::ENOENT;
use std::ffi::OsStr;
use std::time::{Duration, UNIX_EPOCH};

//use syscall::{File, Syscall};

const TTL: Duration = Duration::from_secs(1); // 1 second

const HELLO_DIR_ATTR: FileAttr = FileAttr {
    ino: 1,
    size: 0,
    blocks: 0,
    atime: UNIX_EPOCH,
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::Directory,
    perm: 0o755,
    nlink: 2,
    uid: 501,
    gid: 20,
    rdev: 0,
    flags: 0,
    blksize: 512,
};

const HELLO_TXT_ATTR: FileAttr = FileAttr {
    ino: 2,
    size: 13,
    blocks: 1,
    atime: UNIX_EPOCH, // 1970-01-01 00:00:00
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::RegularFile,
    perm: 0o644,
    nlink: 1,
    uid: 501,
    gid: 20,
    rdev: 0,
    flags: 0,
    blksize: 512,
};


pub struct File {
    fd: i32,
    syscall: Syscall,
}

impl File {
    pub fn new(fd: i32, syscall: Syscall) -> Self {
        File { fd, syscall }
    }

    pub fn read(&self) -> Option<Vec<u8>> {
        let mut req = Syscall::new(TcpStream);
        req.set_dentRead(DentRead { fd: self.fd });

        self.syscall.send(&req);

        if let Ok(response) = self.syscall.recv::<DentResult>() {
            if response.get_success() {
                return Some(response.get_data().to_vec());
            }
        }
        None
    }

    pub fn write(&self, data: Vec<u8>) -> bool {
        let mut update = DentUpdate::new();
        update.set_fd(self.fd);
        update.set_file(data);

        let mut req = Syscall::new();
        req.set_dentUpdate(update);

        self.syscall.send(&req);

        if let Ok(response) = self.syscall.recv::<DentResult>() {
            return response.get_success();
        }
        false
    }
}

impl Syscall {
    pub fn new(sock: TcpStream) -> Self {
        Syscall { sock }
    }

    pub fn send<M: Message>(&mut self, message: &M) {
        let data = message.write_to_bytes().unwrap();
        let size = data.len() as u32;
        self.sock.write_u32::<BigEndian>(size).unwrap();
        self.sock.write_all(&data).unwrap();
    }

    pub fn recv<M: Message + Default>(&self) -> Result<M, std::io::Error> {
        let size = self.sock.read_u32::<BigEndian>()?;
        let mut buf = vec![0; size as usize];
        self.sock.read_exact(&mut buf)?;
        let mut message = M::default();
        message.merge_from_bytes(&buf)?;
        Ok(message)
    }
}

struct HelloFS {
    syscall: Syscall,
}

impl HelloFS {
    fn new(syscall: Syscall) -> Self {
        HelloFS { syscall }
    }
}

impl Filesystem for HelloFS {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        if parent == 1 && name.to_str() == Some("hello.txt") {
            reply.entry(&TTL, &HELLO_TXT_ATTR, 0);
        } else {
            reply.error(ENOENT);
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        match ino {
            1 => reply.attr(&TTL, &HELLO_DIR_ATTR),
            2 => reply.attr(&TTL, &HELLO_TXT_ATTR),
            _ => reply.error(ENOENT),
        }
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        _size: u32,
        _flags: i32,
        _lock: Option<u64>,
        reply: ReplyData,
    ) {
        if ino == 2 {
            let file = File::new(2, self.syscall.clone());
            if let Some(data) = file.read() {
                reply.data(&data[offset as usize..]);
            } else {
                reply.error(ENOENT);
            }
        } else {
            reply.error(ENOENT);
        }
    }

    fn write(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        data: &[u8],
        _flags: i32,
        _lock: Option<u64>,
        reply: ReplyData,
    ) {
        if ino == 2 {
            let file = File::new(2, self.syscall.clone());
            if file.write(data.to_vec()) {
                reply.data(data);
            } else {
                reply.error(ENOENT);
            }
        } else {
            reply.error(ENOENT);
        }
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        if ino != 1 {
            reply.error(ENOENT);
            return;
        }

        let entries = vec![
            (1, FileType::Directory, "."),
            (1, FileType::Directory, ".."),
            (2, FileType::RegularFile, "hello.txt"),
        ];

        for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
            // i + 1 means the index of the next entry
            if reply.add(entry.0, (i + 1) as i64, entry.1, entry.2) {
                break;
            }
        }
        reply.data(data);
    }
}

fn main() {
    let matches = Command::new("hello")
        .version(crate_version!())
        .author("Christopher Berner")
        .arg(
            Arg::new("MOUNT_POINT")
                .required(true)
                .index(1)
                .help("Act as a client, and mount FUSE at given path"),
        )
        .arg(
            Arg::new("auto_unmount")
                .long("auto_unmount")
                .action(ArgAction::SetTrue)
                .help("Automatically unmount on process exit"),
        )
        .arg(
            Arg::new("allow-root")
                .long("allow-root")
                .action(ArgAction::SetTrue)
                .help("Allow root user to access filesystem"),
        )
        .get_matches();
    env_logger::init();
    let mountpoint = matches.get_one::<String>("MOUNT_POINT").unwrap();
    let mut options = vec![MountOption::RO, MountOption::FSName("hello".to_string())];
    if matches.get_flag("auto_unmount") {
        options.push(MountOption::AutoUnmount);
    }
    if matches.get_flag("allow-root") {
        options.push(MountOption::AllowRoot);
    }

    let sock = TcpStream::connect("localhost:12345").unwrap();
    let syscall = Syscall::new(sock);
    let filesystem = HelloFS::new(syscall);
    fuser::mount2(filesystem, mountpoint, &options).unwrap();
}

/* Rust already has third-party crate that allows you to talk to v-sock 
    Establish connection, serialize the object using code generated by protobuf, send serialized files (one communication)
    refer to syscalls.py for python object but do it in more oop style 
    Requirements:
    - Connect fuse to syscalls
    - generate code from protobuf
    - establish connection path with v-sock
    - replicate wrapper objects 

    6/25:
    - for example, focus on one file/blob/gate object 

    7/2: 
    - need to rebuild kernel image
    - use syscalls.proto
    - fuse filesystem is a trait bound to an object/struct. defines some interface.
    - suppose you have a list component - suppose you try to do ls in CLI: it will access the ls, and hopefully will make the syscall
    - generate code from protobuf:
        - go to rootfs, python runtime. make file shows how to generate the code.
    - protobuf will generate basic structure defined in syscalls.proto. but we create wrapper for it to easily construct, send, receive message
    - build.rs: run what you're trying to do in cargo check, so you could try to call syscalls.proto
    - you will get file path to code generated by protobuf once you make. make sure you include the filepath (refer to syscalls.rs, which is a wrapper)
     */

include!(concat!(env!("OUT_DIR"), "/_.rs"));

extern crate vsock;

use clap::{crate_version, Arg, ArgAction, Command};
use fuser::{
    FileAttr, FileType, Filesystem, MountOption, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry
};
use libc::ENOENT;
use std::ffi::OsStr;
use std::io::{Error, Result, Read, Write};
use std::time::{Duration, UNIX_EPOCH};
use prost::Message;
use vsock::{VsockStream};

const TTL: Duration = Duration::from_secs(1); // 1 second

const HELLO_DIR_ATTR: FileAttr = FileAttr {
    ino: 1,
    size: 0,
    blocks: 0,
    atime: UNIX_EPOCH, // 1970-01-01 00:00:00
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

const HELLO_TXT_CONTENT: &str = "Hello World!\n";

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

// interface to talk to vsock device
// create wrapper for protobuf messages
// SyscallClient will take the wrap, dewrap the wrapper or wrap the raw protobuf message and send out
// create vsock connection here
struct SyscallClient {
    sock: VsockStream,
}

impl SyscallClient {
    fn new(cid: u32, port: u32) -> Self {
        let sock = VsockStream::connect_with_cid_port(cid, port)
            .expect("Failed to establish vsock connection");
        Self { sock }
    }

    fn _send(&mut self, obj: &Syscall) -> Result<()> {
        let obj_data = obj.encode_to_vec();
        let length = obj_data.len() as u32; // should have length
        self.sock.write_all(&length.to_be_bytes())?;
        self.sock.write_all(&obj_data)?;
        Ok(())
    }

    fn _recv(&mut self, obj: &Syscall) -> &Syscall {
        let mut buffer = [0; 10];
        let data = self.sock.read_exact(&mut buffer);
        let res = &data.to_be_bytes()?;
        let obj_data = self.sock.read_to_end(res[0]);
        Syscall::decode(obj_data);
        return obj;

    /* 
    def _send(self, obj):
        objData = obj.SerializeToString()
        self.sock.sendall(struct.pack(">I", len(objData)))
        self.sock.sendall(objData)

    def _recv(self, obj):
        data = self.sock.recv(4, socket.MSG_WAITALL) receives first four bytes/length, wait
        res = struct.unpack(">I", data)
        objData = recvall(self.sock, res[0])

        obj.ParseFromString(objData)
        return obj*/
    }
}

struct DirEntry {
    fd: u64,
    client: SyscallClient,
}

struct File {
    entry: DirEntry,
}

impl File {
    fn new(entry: DirEntry) -> Self {
        File { entry }
    }

    fn read(&mut self) -> Result<Option<Vec<u8>> {
        // combine syscall definitions
        let req = Syscall {
            syscall: Some(syscall::Syscall::DentRead(entry.fd)),
        };
        self.entry.client._send(&req)?;

        let response: DentResult = self.entry.client._recv(DentResult());
        if response.success {
            Ok(response.data)
        } else {
            Ok(None)
        }
    }

    fn write(&mut self, data: Vec<u8>) -> bool {
        let req = Syscall {
            syscall: Some(syscall::Syscall::DentUpdate(
                syscall::DentUpdate {
                    fd: entry.fd,
                    kind: Some(dent_update::Kind::File(data)),
                }
            )),
        };
        self.entry.client._send(&req)?;

        let response: DentResult = self.entry.client._recv(DentResult());
        Ok(response.success)
    }

    /* 
    class File(DirEntry):
        def read(self):
            req = syscalls_pb2.Syscall(dentRead=self.fd)
            self.syscall._send(req)
            response = self.syscall._recv(syscalls_pb2.DentResult())
            if response.success:
                return response.data
            else:
                return None

        def write(self, data):
            req = syscalls_pb2.Syscall(dentUpdate=syscalls_pb2.DentUpdate(fd=self.fd, file=data))
            self.syscall._send(req)
            response = self.syscall._recv(syscalls_pb2.DentResult())
            return response.success
    */
}

struct FacetedDirectory {
    entry: DirEntry,
}

impl FacetedDirectory {
    fn ls(&mut self) -> VsockStream {
        let req = Syscall {
            syscall: Some(syscall::Syscall::DentLsFaceted(
                syscall::DentLsFaceted {
                    fd: entry.fd,
                }
            )),
        };
        self.entry.client._send(&req)?;
        let res: DentLsFacetedResult = self.entry.client._recv(DentLsFacetedResult());
        if res != None {
            return None;
        }
        else {
            return None;
        }
    }
}
/* 
class FacetedDirectory(DirEntry):
    def ls(self):
        req = syscalls_pb2.Syscall(dentLsFaceted = syscalls_pb2.DentLsFaceted(fd = self.fd))
        self.syscall._send(req)
        res = self.syscall._recv(syscalls_pb2.DentLsFacetedResult())
        if res is not None:
            return list(map(_Printer()._MessageToJsonObject, res.facets))
        else:
            return None*/

struct BlobEntry {
    entry: DirEntry,
}

impl BlobEntry {
    fn get(&mut self) {
        let req = Syscall {
            syscall: Some(syscall::Syscall::DentGetBlob(entry.fd)),
        };
        self.entry.client._send(&req);
        let response = self.entry.client._recv(BlobResult());
        if response.success {
            // yield Blob(response.fd, response.len, self.syscall)
        }
        else {
            // raise Exception("No such blob")
        }
    }
}

/* 
class BlobEntry(DirEntry):
    @contextmanager
    def get(self):
        req = syscalls_pb2.Syscall(dentGetBlob=self.fd)
        self.syscall._send(req)
        response = self.syscall._recv(syscalls_pb2.BlobResult())
        if response.success:
            
        else:
            raise Exception("No such blob")*/
struct HelloFS {
    client: SyscallClient // otherwise make it global variable
}

// use syscall client to bridge user parameters (e.g. list directory at path a/b -> create syscall request -> get Response from syscall client -> interpret and port to fuser) and faasten system
impl Filesystem for HelloFS {
    fn lookup(&mut self, _req: &fuser::Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        if parent == 1 && name.to_str() == Some("hello.txt") {
            reply.entry(&TTL, &HELLO_TXT_ATTR, 0);
        } else {
            reply.error(ENOENT);
        }
    }

    fn getattr(&mut self, _req: &fuser::Request, ino: u64, reply: ReplyAttr) {
        match ino {
            1 => reply.attr(&TTL, &HELLO_DIR_ATTR),
            2 => reply.attr(&TTL, &HELLO_TXT_ATTR),
            _ => reply.error(ENOENT),
        }
    }

    fn read(
        &mut self,
        _req: &fuser::Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        _size: u32,
        _flags: i32,
        _lock: Option<u64>,
        reply: ReplyData,
    ) {
        if ino == 2 {
            reply.data(&HELLO_TXT_CONTENT.as_bytes()[offset as usize..]);
        } else {
            reply.error(ENOENT);
        }
    }

    fn readdir(
        &mut self,
        _req: &fuser::Request,
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
        reply.ok();
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
    fuser::mount2(HelloFS, mountpoint, &options).unwrap();
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

    // 7/9
    // implement vsock connection here
    // create a syscall object, defined in the code generated by protobuf. serialize syscall object and pass it through the vsock connection
    // serialize means converting object into bytes array. protobuf will automatically implement traits
    // go to target directory to find protobuf code in cli ()
    /*
    the way you could do it is the fuse filesystem is a trait for the filesystem struct (every object will go through the fuse filesystem trait)
    every time you call it you interact with the filesystem structure you created
    because you always have to go through the filesystem interface, you always need the vsock device to talk to the virtual network device which is why you put the vsock connection in the filesystem
    snapfaas.syscalls.rs: bare rust definition of the remote messages
    when you try to make a syscall, it's not a function call: you ahve to create a structure Syscall and pass the syscall to the vsock device. trasmitting from your program to faasten. faasten will get the syscall structure
    from the vsock device and interpret the bytes as a remote message and will realize it's a syscall
    when you want to make a syscall, protobuf already gives you a definition of the syscall, generated rust code is rust enum of syscall. when we make syscall we reuse rust definition and make rust structure. we pass rust syscall structure into syscall.
    after you talk to vsock you get back a response from the protobuf in the rust structure, need to convert it into a valid response of the fuser filesystem
    do a direct translation, try to mimic a normal filesystem
    */

     

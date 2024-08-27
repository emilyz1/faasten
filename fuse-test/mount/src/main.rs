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

    fn read(&mut self) -> Result<Option<Vec<u8>>> {
        // combine syscall definitions
        let req = Syscall {
            syscall: Some(syscall::Syscall::DentRead(self.entry.fd)),
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
                syscall::Syscall::DentUpdate {
                    fd: self.entry.fd,
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
/*
struct FacetedDirectory {
    entry: DirEntry,
}

impl FacetedDirectory {
    fn ls(&mut self) -> Result<> {
        let req = Syscall {
            syscall: Some(syscall::Syscall::DentLsFaceted(
                syscall::Syscall::DentLsFaceted {
                    fd: self.entry.fd,
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
} */
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
/*
struct BlobEntry {
    entry: DirEntry,
}

impl BlobEntry {
    fn get(&mut self) {
        let req = Syscall {
            syscall: Some(syscall::Syscall::DentGetBlob(self.entry.fd)),
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
} */

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
    /* 
    Faasten filesystem:
    an example:
        - faasten has different filesystem objects (type with a label). an example would be in faasten, the objects form a filesystem tree layout. top level will be a root directory, 
        where root directory is a directory while there's a home directory under that's a faceted directory. although the system could be configured such that the root directory is a faceted directory,
        but there are some security issues (no one has root privilege at runtime)
        - directory stores links of other objects (file objects, directories, gate objects, etc.). a link (id) is an identifier of an object. each object has a unique id/soft link. kind of like a linked list of ids
        - backend storage is a key-value storage with a random number generator for id for the key for the object
        - fd: file descriptor (virtualized object id, local id at runtime. fd translated to actual unique id for security)
        - suppose we have a file system with a root directory, home directory (faceted directory under root directory) - we have two objects: root directory object (id is 0), home faceted directory. root directory stores the id of the home directory
        - above the faasten filesystem, we have cloud calls/syscalls: system that allows us to modify/operate on objects 
            - (e.g. syscall called list. by giving this syscall a directory, returns a list of fds of objects under the directory). first create a syscall message, pass it to the vsock, vsock will return result and we just need to interpret the result
            - suppose we want to list a path. first we need to get the fd of the path /a/b/c. we need to get fd of /a, but we know root is /. we make a list syscall on root directory (/) and get the fd of /a. use the result to get /a/b and /a/b/c
        - files are objects that store raw bytes. internally in the key-value store, the value is gonna be some bytes in the file. blobs don't store raw bytes - stores the hash of its content. to get blob content, use hash to get the content by querying another database
    */
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
    // build a kernel image (kernel code/linux) with fuse.ko (but does not include filesystem. no disks.)
    // in faasten we use docker to generate the filesystem image (acts as a disk with filesystem layout)
    // thus, we need to have a filesystem installed to have a basic filesystem structure
    // now you have the base, still need a runtime image with libfuse to interpret code
    // then we ne need a docker image to prepare the libfuse
    // suppose we have a vm, and it only has the faasten filesystem and interpreter. second requirement to automatically run the code
    // in the runtime filesystem, we use the system service (every time the os boots up, automatically execute some script, runs some python code we provide)
    // executes some function and returns the result.
    // some things i should know:
    // - the vm we're using is from firecracker which is from amazon in rust. if you go to their github repo there should be some resource directories.
    // - to build the kernel image you need to have a configuration file: defines some configuration including what kind of kernel module it will include
    // - modify given firecracker configuration file to have fuse.ko enabled. then you can just try to build it
    // - need to understand how to build the kernel image, enable fuse.ko, rebuild. once you have the kernel image built, first step is done.
    // - second part is to have libfuse installed. then you can look at the docker. we use the docker to build the filesystem. docker will boostrap filesystem for you
    // - use docker to help install some libraries/packages. then you can export its filesystem outside to a separate image (which is what we want)
    // - look at how the filesystem is built, modify some docker files to have libfuse library installed, export to the filesystem.
    // 
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

     

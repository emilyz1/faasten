use core::panic;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use log::{warn, debug};

use super::message;
use super::resource_manager::ResourceManager;
use super::rpc::ResourceInfo;
use super::Task;

pub type Manager = Arc<Mutex<ResourceManager>>;

pub struct RpcServer {
    manager: Manager,
    listener: TcpListener,
    queue_rx: crossbeam::channel::Receiver<Task>,
    queue_tx: crossbeam::channel::Sender<Task>,
}

impl RpcServer {
    pub fn new(addr: &str, manager: Manager, queue_size: usize) -> Self {
        let (queue_tx, queue_rx) = crossbeam::channel::bounded(queue_size);
        Self {
            manager,
            listener: TcpListener::bind(addr).expect("bind to the TCP listening address"),
            queue_rx,
            queue_tx,
        }
    }

    pub fn run(self) {
        loop {
            for stream in self.listener.incoming() {
                if let Ok(stream) = stream {
                    debug!("connection from {:?}", stream.peer_addr());
                    let manager = Arc::clone(&self.manager);
                    let queue_tx = self.queue_tx.clone();
                    let queue_rx = self.queue_rx.clone();

                    thread::spawn(move || RpcServer::serve(stream, manager, queue_tx, queue_rx));
                }
            }
        }
    }

    // Process the RPC request
    fn serve(
        mut stream: TcpStream, manager: Manager,
        queue_tx: crossbeam::channel::Sender<Task>,
        queue_rx: crossbeam::channel::Receiver<Task>,
    ) {
        while let Ok(req) = message::read_request(&mut stream) {
            use message::{request::Kind, Response, response::Kind as ResKind};
            match req.kind {
                Some(Kind::Ping(_)) => {
                    debug!("PING");
                    let res = Response {
                        kind: Some(ResKind::Pong(message::Pong {})),
                    };
                    let _ = message::write(&mut stream, &res);
                }
                Some(Kind::GetTask(r)) => {
                    debug!("RPC GET received {:?}", r.thread_id);
                    if let Ok(task) = queue_rx.recv() {
                        match task {
                            Task::Invoke(uuid, labeled_invoke) => {
                                let res = message::Response {
                                    kind: Some(ResKind::ProcessTask(message::ProcessTask {
                                        task_id: uuid.to_string(),
                                        labeled_invoke: Some(labeled_invoke),
                                    })),
                                };
                                let _ = message::write(&mut stream, &res);
                            }
                            Task::Terminate => {
                                let res = Response {
                                    kind: Some(ResKind::Terminate(message::Terminate {})),
                                };
                                let _ = message::write(&mut stream, &res);
                            }
                        }
                    }
                }
                Some(Kind::FinishTask(r)) => {
                    debug!("RPC FINISH received {:?}", r.result);
                    let res = Response { kind: None };
                    let _ = message::write(&mut stream, &res);
                    let result = r.result.unwrap();
                    if let Ok(uuid) = uuid::Uuid::parse_str(&r.task_id) {
                        if !uuid.is_nil() {
                            let mut manager = manager.lock().unwrap();
                            if let Some(tx) = manager.wait_list.remove(&uuid) {
                                let _ = tx.send(result);
                            }
                        }
                    }
                }
                Some(Kind::LabeledInvoke(r)) => {
                    debug!("RPC INVOKE received {:?}", r);
                    let uuid = uuid::Uuid::new_v4();
                    let sync = r.sync;
                    match queue_tx.try_send(Task::Invoke(uuid, r)) {
                        Err(crossbeam::channel::TrySendError::Full(_)) => {
                            warn!("Dropping Invocation from {:?}", stream.peer_addr());
                            let ret = message::TaskReturn {
                                code: message::ReturnCode::QueueFull as i32,
                                payload: None,
                            };
                            let _ = message::write(&mut stream, &ret);
                        }
                        Err(crossbeam::channel::TrySendError::Disconnected(_)) =>
                            panic!("Broken request queue"),
                        Ok(()) => {
                            if sync {
                                let (sync_invoke_s, sync_invoke_r) = std::sync::mpsc::channel();
                                manager.lock().unwrap().wait_list.insert(uuid, sync_invoke_s);
                                let ret = sync_invoke_r.recv().unwrap();
                                let _ = message::write(&mut stream, &ret);
                            } else {
                                let ret = message::TaskReturn {
                                    code: message::ReturnCode::Success as i32,
                                    payload: None,
                                };
                                let _ = message::write(&mut stream, &ret);
                            }
                        }
                    }
                }
                //Some(Kind::TerminateAll(_)) => {
                //    debug!("RPC TERMINATEALL received");
                //    let _ = manager.lock().unwrap().remove_all();
                //    let res = Response { kind: None };
                //    let _ = message::write(&mut stream, &res);
                //    break;
                //}
                Some(Kind::UpdateResource(r)) => {
                    debug!("RPC UPDATE received");
                    let manager = &mut manager.lock().unwrap();
                    let info = serde_json::from_slice::<ResourceInfo>(&r.info);
                    if let Ok(info) = info {
                        let addr = stream.peer_addr().unwrap().ip();
                        manager.update(addr, info);
                        let res = Response { kind: None };
                        let _ = message::write(&mut stream, &res);
                    } else {
                        // TODO Send error code
                        let res = Response { kind: None };
                        let _ = message::write(&mut stream, &res);
                    }
                }
                Some(Kind::DropResource(_)) => {
                    debug!("RPC DROP received");
                    let manager = &mut manager.lock().unwrap();
                    let addr = stream.peer_addr().unwrap().ip();
                    manager.remove(addr);
                    let res = Response { kind: None };
                    let _ = message::write(&mut stream, &res);
                    break;
                }
                _ => {}
            }
        }
    }
}

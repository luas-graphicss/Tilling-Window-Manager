use std::fs::{self, DirBuilder};
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use tilling_ipc::endpoint::{InstanceName, RuntimeDirectory, SOCKET_VARIABLE};
use tilling_ipc::server::{Connection, Listener};
use tilling_ipc::{Outcome, Request, Response, ServerMessage};

pub type Answer = fn(&Request) -> Vec<ServerMessage>;

const POLL_INTERVAL: Duration = Duration::from_millis(1);

static NEXT_RUNTIME: AtomicU32 = AtomicU32::new(0);

pub struct Runtime {
    root: PathBuf,
}

impl Runtime {
    pub fn new() -> Self {
        let serial = NEXT_RUNTIME.fetch_add(1, Ordering::Relaxed);
        let name = format!("tillingctl-{}-{serial}", std::process::id());
        let root = std::env::temp_dir().join(name);
        DirBuilder::new()
            .mode(0o700)
            .create(&root)
            .unwrap_or_else(|error| panic!("create {}: {error}", root.display()));
        Self { root }
    }

    pub fn start(&self, instance: &str, answer: Answer) -> Instance {
        let name: InstanceName = instance
            .parse()
            .unwrap_or_else(|error| panic!("instance name {instance}: {error}"));
        let directory = RuntimeDirectory::open(self.root.clone())
            .unwrap_or_else(|error| panic!("open runtime directory: {error}"));
        let socket = directory.socket_path(&name);
        let listener =
            Listener::bind(&socket).unwrap_or_else(|error| panic!("bind {instance}: {error}"));
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        let worker = thread::spawn(move || serve(&listener, answer, &stopped));
        Instance {
            socket,
            stop,
            worker: Some(worker),
        }
    }

    pub fn tillingctl(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_tillingctl"));
        command
            .env("XDG_RUNTIME_DIR", &self.root)
            .env_remove(SOCKET_VARIABLE);
        command
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub struct Instance {
    socket: PathBuf,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Instance {
    pub fn socket(&self) -> &Path {
        &self.socket
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

pub fn reply(request: &Request, outcome: Outcome) -> ServerMessage {
    ServerMessage::Response(Response {
        request_id: request.request_id,
        outcome,
    })
}

pub fn standard_output(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub fn standard_error(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn serve(listener: &Listener, answer: Answer, stopped: &AtomicBool) {
    let mut connections = Vec::new();
    while !stopped.load(Ordering::Relaxed) {
        if let Ok(Some(connection)) = listener.accept() {
            connections.push(connection);
        }
        connections.retain_mut(|connection| exchange(connection, answer));
        thread::sleep(POLL_INTERVAL);
    }
}

fn exchange(connection: &mut Connection, answer: Answer) -> bool {
    let Ok(requests) = connection.receive(usize::MAX) else {
        return false;
    };
    let queued = requests
        .iter()
        .flat_map(answer)
        .all(|message| connection.send(&message).is_ok());
    queued && connection.flush().is_ok()
}

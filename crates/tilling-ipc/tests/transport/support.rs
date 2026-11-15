use std::collections::VecDeque;
use std::fs::{self, DirBuilder};
use std::io::{Read, Write};
use std::os::unix::fs::DirBuilderExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde::de::DeserializeOwned;
use tilling_ipc::server::{Connection, Listener};
use tilling_ipc::{
    Call, Capability, FrameDecoder, HandshakeReply, Hello, PROTOCOL_VERSION, ProtocolVersion,
    Request, ServerMessage, decode, encode,
};

pub const PATIENCE: Duration = Duration::from_secs(5);

const POLL_INTERVAL: Duration = Duration::from_millis(1);
const READ_CHUNK_BYTES: usize = 4096;

static NEXT_SCRATCH: AtomicU32 = AtomicU32::new(0);

pub struct Scratch {
    root: PathBuf,
}

impl Scratch {
    pub fn new() -> Self {
        let serial = NEXT_SCRATCH.fetch_add(1, Ordering::Relaxed);
        let name = format!("tilling-ipc-{}-{serial}", std::process::id());
        let root = std::env::temp_dir().join(name);
        DirBuilder::new()
            .mode(0o700)
            .create(&root)
            .unwrap_or_else(|error| panic!("create {}: {error}", root.display()));
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn socket(&self) -> PathBuf {
        self.root.join("tilling/test/ipc.sock")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub struct Peer {
    stream: UnixStream,
    decoder: FrameDecoder,
    frames: VecDeque<Vec<u8>>,
}

impl Peer {
    pub fn connect(socket: &Path) -> Self {
        let stream = UnixStream::connect(socket)
            .unwrap_or_else(|error| panic!("connect to {}: {error}", socket.display()));
        stream
            .set_read_timeout(Some(PATIENCE))
            .unwrap_or_else(|error| panic!("set read timeout: {error}"));
        Self {
            stream,
            decoder: FrameDecoder::default(),
            frames: VecDeque::new(),
        }
    }

    pub fn write(&mut self, bytes: &[u8]) {
        self.stream
            .write_all(bytes)
            .unwrap_or_else(|error| panic!("write to the server: {error}"));
    }

    pub fn send(&mut self, message: &impl Serialize) {
        let frame = encode(message).unwrap_or_else(|error| panic!("encode message: {error}"));
        self.write(&frame);
    }

    pub fn greet(&mut self) {
        self.send(&hello(PROTOCOL_VERSION));
    }

    pub fn receive<T: DeserializeOwned>(&mut self) -> T {
        loop {
            if let Some(frame) = self.frames.pop_front() {
                return decode(&frame)
                    .unwrap_or_else(|error| panic!("decode server message: {error}"));
            }
            let mut chunk = [0_u8; READ_CHUNK_BYTES];
            let length = self
                .stream
                .read(&mut chunk)
                .unwrap_or_else(|error| panic!("read from the server: {error}"));
            assert_ne!(length, 0, "server closed the connection");
            let frames = self
                .decoder
                .push(chunk.get(..length).unwrap_or_default())
                .unwrap_or_else(|error| panic!("server broke the framing: {error}"));
            self.frames.extend(frames);
        }
    }
}

pub struct Session {
    pub connection: Connection,
    pub peer: Peer,
    _listener: Listener,
    _scratch: Scratch,
}

pub struct Background {
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Drop for Background {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

pub fn hello(protocol: ProtocolVersion) -> Hello {
    Hello {
        protocol,
        capabilities: vec![Capability::Events.name().to_owned()],
    }
}

pub fn request(request_id: u64, call: Call) -> Request {
    Request {
        request_id,
        expected_revision: None,
        call,
    }
}

pub fn eventually<T>(mut attempt: impl FnMut() -> Option<T>) -> T {
    let deadline = Instant::now() + PATIENCE;
    loop {
        if let Some(found) = attempt() {
            return found;
        }
        assert!(
            Instant::now() < deadline,
            "nothing happened within {PATIENCE:?}"
        );
        thread::sleep(POLL_INTERVAL);
    }
}

pub fn bind(scratch: &Scratch) -> Listener {
    Listener::bind(&scratch.socket()).unwrap_or_else(|error| panic!("bind listener: {error}"))
}

pub fn accept(listener: &Listener) -> Connection {
    eventually(|| {
        listener
            .accept()
            .unwrap_or_else(|error| panic!("accept peer: {error}"))
    })
}

pub fn receive(connection: &mut Connection, count: usize) -> Vec<Request> {
    let mut requests = Vec::new();
    eventually(|| {
        let batch = connection
            .receive(count - requests.len())
            .unwrap_or_else(|error| panic!("receive requests: {error}"));
        requests.extend(batch);
        (requests.len() == count).then_some(())
    });
    requests
}

pub fn flush(connection: &mut Connection) {
    eventually(|| {
        connection
            .flush()
            .unwrap_or_else(|error| panic!("flush replies: {error}"))
            .then_some(())
    });
}

pub fn established() -> Session {
    let scratch = Scratch::new();
    let listener = bind(&scratch);
    let mut peer = Peer::connect(&scratch.socket());
    let mut connection = accept(&listener);
    peer.greet();
    receive(&mut connection, 0);
    eventually(|| {
        receive(&mut connection, 0);
        connection.grants(Capability::Events).then_some(())
    });
    flush(&mut connection);
    let _: HandshakeReply = peer.receive();
    Session {
        connection,
        peer,
        _listener: listener,
        _scratch: scratch,
    }
}

pub fn serve(listener: Listener, answer: fn(&Request) -> Vec<ServerMessage>) -> Background {
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = Arc::clone(&stop);
    let worker = thread::spawn(move || {
        let mut connections = Vec::new();
        while !stopped.load(Ordering::Relaxed) {
            if let Ok(Some(connection)) = listener.accept() {
                connections.push(connection);
            }
            connections.retain_mut(|connection| exchange(connection, answer));
            thread::sleep(POLL_INTERVAL);
        }
    });
    Background {
        stop,
        worker: Some(worker),
    }
}

fn exchange(connection: &mut Connection, answer: fn(&Request) -> Vec<ServerMessage>) -> bool {
    let Ok(requests) = connection.receive(usize::MAX) else {
        return false;
    };
    let queued = requests
        .iter()
        .flat_map(answer)
        .all(|message| connection.send(&message).is_ok());
    queued && connection.flush().is_ok()
}

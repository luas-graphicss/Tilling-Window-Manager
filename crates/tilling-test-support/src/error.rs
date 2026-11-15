use std::path::PathBuf;

use wayland_client::globals::{BindError, GlobalError};
use wayland_client::{ConnectError, DispatchError};

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("connect to {}", .path.display())]
    Connect {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("start the wayland connection")]
    Handshake(#[source] ConnectError),
    #[error("list the advertised globals")]
    Globals(#[source] GlobalError),
    #[error("bind {interface}")]
    Bind {
        interface: &'static str,
        #[source]
        source: BindError,
    },
    #[error("dispatch wayland events")]
    Dispatch(#[source] DispatchError),
    #[error("allocate a shared memory buffer")]
    Buffer(#[source] rustix::io::Errno),
    #[error("buffer of {width}x{height} pixels exceeds the shared memory limits")]
    BufferTooLarge { width: u32, height: u32 },
    #[error("toplevel was already destroyed")]
    DestroyedToplevel,
    #[error("no pointer button event was received yet")]
    NoButtonSerial,
    #[error("timed out waiting for {0}")]
    Timeout(&'static str),
}

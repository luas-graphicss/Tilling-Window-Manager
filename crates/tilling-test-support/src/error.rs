use std::path::PathBuf;

use wayland_client::backend::WaylandError;
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
    #[error("popup was already destroyed")]
    DestroyedPopup,
    #[error("sub-surface was already destroyed")]
    DestroyedSubsurface,
    #[error("layer surface was already destroyed")]
    DestroyedLayer,
    #[error("no clipboard content is offered to this client")]
    NoClipboardOffer,
    #[error("this client holds no session lock")]
    NoSessionLock,
    #[error("open a pipe for a data transfer")]
    Pipe(#[source] std::io::Error),
    #[error("read the transferred data")]
    Transfer(#[source] std::io::Error),
    #[error("no pointer button event was received yet")]
    NoButtonSerial,
    #[error("pointer has not entered a surface of this client")]
    PointerOutside,
    #[error("compositor does not negotiate decorations")]
    NoDecorationManager,
    #[error("compositor does not report fractional scales")]
    NoFractionalScaleManager,
    #[error("timed out waiting for {0}")]
    Timeout(&'static str),
}

impl ClientError {
    #[must_use]
    pub fn protocol_error(&self) -> Option<(&str, u32)> {
        match self {
            Self::Dispatch(DispatchError::Backend(WaylandError::Protocol(error))) => {
                Some((error.object_interface.as_str(), error.code))
            }
            _ => None,
        }
    }
}

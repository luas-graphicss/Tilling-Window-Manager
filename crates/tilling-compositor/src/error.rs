use std::error::Error;
use std::path::PathBuf;

#[cfg(feature = "drm")]
use smithay::backend::drm::CreateDrmNodeError;
use smithay::reexports::calloop;
use smithay::reexports::wayland_server::BindError;
use smithay::reexports::wayland_server::backend::InitError;
use tilling_core::TransitionError;

use crate::runtime::instance::InstanceError;

#[derive(Debug, thiserror::Error)]
pub enum CompositorError {
    #[error("create the event loop")]
    EventLoop(#[source] calloop::Error),
    #[error("register an event source")]
    EventSource(#[source] calloop::Error),
    #[error("dispatch the event loop")]
    Dispatch(#[source] calloop::Error),
    #[error("watch termination signals")]
    Signals(#[source] std::io::Error),
    #[error("create the wayland display")]
    Display(#[source] InitError),
    #[error("duplicate the wayland display descriptor")]
    DisplayDescriptor(#[source] std::io::Error),
    #[error("bind the wayland socket {}", .path.display())]
    Socket {
        path: PathBuf,
        #[source]
        source: BindError,
    },
    #[error("load the keyboard layout")]
    Keyboard(#[source] smithay::input::keyboard::Error),
    #[error("prepare the initial desktop")]
    Desktop(#[from] TransitionError),
    #[error("configured workspaces are empty")]
    NoWorkspaces,
    #[error("virtual output {width}x{height} is not a usable size")]
    OutputSize { width: u32, height: u32 },
    #[error("output {0} is already connected")]
    OutputInUse(String),
    #[error("output {0} is not connected")]
    UnknownOutput(String),
    #[error("the {0} backend does not take virtual outputs")]
    FixedOutputs(&'static str),
    #[error("this build does not include the {0} backend")]
    BackendUnavailable(&'static str),
    #[error("the {0} backend needs the main thread and cannot run in a session")]
    MainThreadBackend(&'static str),
    #[cfg(feature = "nested")]
    #[error("open the host window: {0}")]
    HostWindow(String),
    #[cfg(feature = "drm")]
    #[error("open the seat session: {0}")]
    Seat(String),
    #[cfg(feature = "drm")]
    #[error("open the input devices of seat {0}")]
    InputSeat(String),
    #[cfg(feature = "drm")]
    #[error("enumerate graphics devices")]
    Gpus(#[source] std::io::Error),
    #[cfg(feature = "drm")]
    #[error("identify graphics device {}", .path.display())]
    GpuNode {
        path: PathBuf,
        #[source]
        source: CreateDrmNodeError,
    },
    #[cfg(feature = "drm")]
    #[error("no usable graphics device on seat {0}")]
    NoGpu(String),
    #[cfg(feature = "drm")]
    #[error("start the renderer: {0}")]
    Renderer(String),
    #[error(transparent)]
    Instance(#[from] InstanceError),
    #[error("open the control socket")]
    ControlSocket(#[from] tilling_ipc::server::BindError),
    #[error("start the compositor thread")]
    SessionThread(#[source] std::io::Error),
    #[error("compositor session is no longer running")]
    SessionClosed,
    #[error("open a privileged client connection")]
    PrivilegedClient(#[source] std::io::Error),
}

pub(crate) fn cause_chain(error: &dyn Error) -> String {
    let mut description = error.to_string();
    let mut cause = error.source();
    while let Some(source) = cause {
        description.push_str(": ");
        description.push_str(&source.to_string());
        cause = source.source();
    }
    description
}

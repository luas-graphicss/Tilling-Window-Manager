use std::path::PathBuf;

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
    Signals(#[source] calloop::Error),
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
    #[error("this build does not include the {0} backend")]
    BackendUnavailable(&'static str),
    #[error(transparent)]
    Instance(#[from] InstanceError),
    #[error("start the compositor thread")]
    SessionThread(#[source] std::io::Error),
    #[error("compositor session is no longer running")]
    SessionClosed,
}

use std::io;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use smithay::reexports::calloop::generic::Generic;
use smithay::reexports::calloop::{Interest, LoopHandle, Mode, PostAction};
use smithay::reexports::wayland_server::backend::{ClientData, ClientId, DisconnectReason};
use smithay::reexports::wayland_server::{Display, ListeningSocket};
use smithay::wayland::compositor::CompositorClientState;

use super::{Compositor, Runtime};
use crate::error::CompositorError;
use crate::security::Privileges;

#[derive(Debug, Default)]
pub(crate) struct ClientState {
    pub(crate) compositor_state: CompositorClientState,
    pub(crate) privileges: Privileges,
    departures: Departures,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Departures(Arc<AtomicBool>);

impl ClientData for ClientState {
    fn disconnected(&self, _client: ClientId, _reason: DisconnectReason) {
        self.departures.0.store(true, Ordering::Relaxed);
    }
}

impl Departures {
    pub(crate) fn take(&self) -> bool {
        self.0.swap(false, Ordering::Relaxed)
    }
}

impl Compositor {
    pub(crate) fn connect_privileged(&mut self, privileges: Privileges) -> io::Result<UnixStream> {
        let (accepted, offered) = UnixStream::pair()?;
        self.accept_client(accepted, privileges)?;
        Ok(offered)
    }

    fn accept_client(&mut self, stream: UnixStream, privileges: Privileges) -> io::Result<()> {
        let client = Arc::new(ClientState {
            compositor_state: CompositorClientState::default(),
            privileges,
            departures: self.departures.clone(),
        });
        self.display_handle.insert_client(stream, client)?;
        Ok(())
    }
}

pub(crate) fn listen(
    handle: &LoopHandle<'static, Runtime>,
    path: &Path,
) -> Result<(), CompositorError> {
    let socket = ListeningSocket::bind_absolute(path.to_path_buf()).map_err(|source| {
        CompositorError::Socket {
            path: path.to_path_buf(),
            source,
        }
    })?;
    let source = Generic::new(socket, Interest::READ, Mode::Level);
    handle
        .insert_source(source, |_, socket, runtime| {
            while let Some(stream) = socket.accept()? {
                let accepted = runtime
                    .compositor
                    .accept_client(stream, Privileges::default());
                if let Err(error) = accepted {
                    tracing::warn!(%error, "refuse wayland client");
                }
            }
            Ok(PostAction::Continue)
        })
        .map_err(|error| CompositorError::EventSource(error.error))?;
    Ok(())
}

pub(crate) fn watch_display(
    handle: &LoopHandle<'static, Runtime>,
    display: &mut Display<Compositor>,
) -> Result<(), CompositorError> {
    let descriptor = display
        .backend()
        .poll_fd()
        .try_clone_to_owned()
        .map_err(CompositorError::DisplayDescriptor)?;
    let source = Generic::new(descriptor, Interest::READ, Mode::Level);
    handle
        .insert_source(source, |_, _, runtime| {
            runtime.display.dispatch_clients(&mut runtime.compositor)?;
            Ok(PostAction::Continue)
        })
        .map_err(|error| CompositorError::EventSource(error.error))?;
    Ok(())
}

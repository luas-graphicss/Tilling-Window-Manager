use std::path::Path;
use std::sync::Arc;

use smithay::reexports::calloop::generic::Generic;
use smithay::reexports::calloop::{Interest, LoopHandle, Mode, PostAction};
use smithay::reexports::wayland_server::backend::ClientData;
use smithay::reexports::wayland_server::{Display, ListeningSocket};
use smithay::wayland::compositor::CompositorClientState;

use super::{Compositor, Runtime};
use crate::error::CompositorError;

#[derive(Debug, Default)]
pub(crate) struct ClientState {
    pub(crate) compositor_state: CompositorClientState,
}

impl ClientData for ClientState {}

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
                let client = Arc::new(ClientState::default());
                let inserted = runtime
                    .compositor
                    .display_handle
                    .insert_client(stream, client);
                if let Err(error) = inserted {
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

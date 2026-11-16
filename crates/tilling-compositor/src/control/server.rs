use std::os::fd::AsFd;
use std::path::Path;

use smithay::reexports::calloop::generic::Generic;
use smithay::reexports::calloop::{
    self, Interest, LoopHandle, Mode, PostAction, RegistrationToken,
};
use tilling_ipc::ServerMessage;
use tilling_ipc::server::{AcceptError, Connection, Listener};

use super::peers::{MAX_PEERS, PeerId};
use crate::error::CompositorError;
use crate::runtime::{Compositor, Runtime};

pub(crate) fn listen(
    handle: &LoopHandle<'static, Runtime>,
    socket: &Path,
) -> Result<(), CompositorError> {
    let listener = Listener::bind(socket)?;
    let source = Generic::new(listener, Interest::READ, Mode::Level);
    handle
        .insert_source(source, |_, listener, runtime| {
            runtime.compositor.accept_peers(listener);
            Ok(PostAction::Continue)
        })
        .map_err(|error| CompositorError::EventSource(error.error))?;
    Ok(())
}

impl Compositor {
    pub(super) fn reply_to(&mut self, id: PeerId, message: &ServerMessage) {
        let Some(peer) = self.control.get_mut(id) else {
            return;
        };
        if let Err(error) = peer.connection.send(message) {
            tracing::debug!(%error, "drop control peer that stopped reading");
            self.drop_peer(id);
        }
    }

    pub(super) fn flush_peer(&mut self, id: PeerId) {
        let Some(peer) = self.control.get_mut(id) else {
            return;
        };
        match peer.connection.flush() {
            Ok(true) => {}
            Ok(false) => self.await_writable(id),
            Err(error) => {
                tracing::debug!(%error, "control peer went away");
                self.drop_peer(id);
            }
        }
    }

    pub(super) fn drop_peer(&mut self, id: PeerId) {
        let Some(peer) = self.control.remove(id) else {
            return;
        };
        self.loop_handle.remove(peer.reader);
        if let Some(writer) = peer.writer {
            self.loop_handle.remove(writer);
        }
    }

    fn accept_peers(&mut self, listener: &Listener) {
        loop {
            match listener.accept() {
                Ok(Some(connection)) => self.admit(connection),
                Ok(None) => return,
                Err(AcceptError::Peer(error)) => tracing::warn!(%error, "refuse control peer"),
                Err(AcceptError::Socket(error)) => {
                    tracing::warn!(%error, "accept control peer");
                    return;
                }
            }
        }
    }

    fn admit(&mut self, connection: Connection) {
        if self.control.is_full() {
            tracing::warn!(limit = MAX_PEERS, "refuse control peer beyond the limit");
            return;
        }
        let id = self.control.reserve();
        let watched = watch(
            &self.loop_handle,
            &connection,
            Interest::READ,
            move |compositor| compositor.peer_readable(id),
        );
        match watched {
            Ok(reader) => self.control.insert(id, connection, reader),
            Err(error) => tracing::warn!(%error, "watch control peer"),
        }
    }

    fn peer_readable(&mut self, id: PeerId) -> PostAction {
        let Some(peer) = self.control.get_mut(id) else {
            return PostAction::Remove;
        };
        let requests = match peer.connection.receive(usize::MAX) {
            Ok(requests) => requests,
            Err(error) => {
                tracing::debug!(%error, "close control peer");
                self.drop_peer(id);
                return PostAction::Remove;
            }
        };
        for request in requests {
            if let Some(response) = self.answer(id, request) {
                self.reply_to(id, &ServerMessage::Response(response));
            }
        }
        self.flush_peer(id);
        PostAction::Continue
    }

    fn peer_writable(&mut self, id: PeerId) -> PostAction {
        let Some(peer) = self.control.get_mut(id) else {
            return PostAction::Remove;
        };
        match peer.connection.flush() {
            Ok(true) => {
                peer.writer = None;
                PostAction::Remove
            }
            Ok(false) => PostAction::Continue,
            Err(error) => {
                tracing::debug!(%error, "control peer went away");
                peer.writer = None;
                self.drop_peer(id);
                PostAction::Remove
            }
        }
    }

    fn await_writable(&mut self, id: PeerId) {
        let Some(peer) = self.control.get_mut(id) else {
            return;
        };
        if peer.writer.is_some() {
            return;
        }
        let watched = watch(
            &self.loop_handle,
            &peer.connection,
            Interest::WRITE,
            move |compositor| compositor.peer_writable(id),
        );
        match watched {
            Ok(writer) => peer.writer = Some(writer),
            Err(error) => {
                tracing::warn!(%error, "watch congested control peer");
                self.drop_peer(id);
            }
        }
    }
}

fn watch(
    handle: &LoopHandle<'static, Runtime>,
    connection: &Connection,
    interest: Interest,
    mut react: impl FnMut(&mut Compositor) -> PostAction + 'static,
) -> Result<RegistrationToken, calloop::Error> {
    let descriptor = connection.as_fd().try_clone_to_owned()?;
    let source = Generic::new(descriptor, interest, Mode::Level);
    handle
        .insert_source(source, move |_, _, runtime| {
            Ok(react(&mut runtime.compositor))
        })
        .map_err(|error| error.error)
}

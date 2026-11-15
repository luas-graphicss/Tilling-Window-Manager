use wayland_client::{Connection, Dispatch, QueueHandle, WEnum, delegate_noop};
use wayland_protocols::xdg::decoration::zv1::client::zxdg_decoration_manager_v1::ZxdgDecorationManagerV1;
use wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1::{
    self, Mode, ZxdgToplevelDecorationV1,
};

use crate::client::{Client, ToplevelId};
use crate::error::ClientError;
use crate::state::ClientState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecorationMode {
    ClientSide,
    ServerSide,
}

delegate_noop!(ClientState: ZxdgDecorationManagerV1);

impl Dispatch<ZxdgToplevelDecorationV1, usize> for ClientState {
    fn event(
        state: &mut Self,
        _decoration: &ZxdgToplevelDecorationV1,
        event: zxdg_toplevel_decoration_v1::Event,
        toplevel: &usize,
        _connection: &Connection,
        _queue: &QueueHandle<Self>,
    ) {
        let zxdg_toplevel_decoration_v1::Event::Configure {
            mode: WEnum::Value(mode),
        } = event
        else {
            return;
        };
        if let Some(record) = state.toplevels.get_mut(*toplevel) {
            record.decoration = match mode {
                Mode::ServerSide => Some(DecorationMode::ServerSide),
                Mode::ClientSide => Some(DecorationMode::ClientSide),
                _ => None,
            };
        }
    }
}

impl Client {
    #[must_use]
    pub fn decoration_mode(&self, toplevel: ToplevelId) -> Option<DecorationMode> {
        self.state.toplevels.get(toplevel.index())?.decoration
    }

    pub fn request_decoration(
        &mut self,
        toplevel: ToplevelId,
        mode: DecorationMode,
    ) -> Result<(), ClientError> {
        let manager = self
            .decoration_manager
            .as_ref()
            .ok_or(ClientError::NoDecorationManager)?;
        let role = self.toplevel_role(toplevel)?;
        let decoration =
            manager.get_toplevel_decoration(role, &self.queue.handle(), toplevel.index());
        decoration.set_mode(match mode {
            DecorationMode::ClientSide => Mode::ClientSide,
            DecorationMode::ServerSide => Mode::ServerSide,
        });
        self.decorations.push((toplevel, decoration));
        Ok(())
    }

    pub(crate) fn release_decoration(&mut self, toplevel: ToplevelId) {
        self.decorations.retain(|(owner, decoration)| {
            if *owner == toplevel {
                decoration.destroy();
            }
            *owner != toplevel
        });
    }
}

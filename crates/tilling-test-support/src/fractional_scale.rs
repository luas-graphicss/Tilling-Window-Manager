use wayland_client::{Connection, Dispatch, QueueHandle, delegate_noop};
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1;
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_v1::{
    self, WpFractionalScaleV1,
};

use crate::client::{Client, ToplevelId};
use crate::error::ClientError;
use crate::state::ClientState;

delegate_noop!(ClientState: WpFractionalScaleManagerV1);

impl Dispatch<WpFractionalScaleV1, usize> for ClientState {
    fn event(
        state: &mut Self,
        _watcher: &WpFractionalScaleV1,
        event: wp_fractional_scale_v1::Event,
        toplevel: &usize,
        _connection: &Connection,
        _queue: &QueueHandle<Self>,
    ) {
        let wp_fractional_scale_v1::Event::PreferredScale { scale } = event else {
            return;
        };
        if let Some(record) = state.toplevels.get_mut(*toplevel) {
            record.preferred_scale = Some(scale);
        }
    }
}

impl Client {
    #[must_use]
    pub fn preferred_scale(&self, toplevel: ToplevelId) -> Option<u32> {
        self.state.toplevels.get(toplevel.index())?.preferred_scale
    }

    pub fn watch_preferred_scale(&mut self, toplevel: ToplevelId) -> Result<(), ClientError> {
        let manager = self
            .fractional_scale_manager
            .as_ref()
            .ok_or(ClientError::NoFractionalScaleManager)?;
        let surface = self.toplevel_surface(toplevel)?;
        let watcher = manager.get_fractional_scale(surface, &self.queue.handle(), toplevel.index());
        self.scale_watchers.push((toplevel, watcher));
        Ok(())
    }

    pub(crate) fn release_preferred_scale(&mut self, toplevel: ToplevelId) {
        self.scale_watchers.retain(|(owner, watcher)| {
            if *owner == toplevel {
                watcher.destroy();
            }
            *owner != toplevel
        });
    }
}

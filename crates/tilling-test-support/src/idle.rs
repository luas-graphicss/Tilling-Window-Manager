use std::time::Duration;

use wayland_client::{Connection, Dispatch, QueueHandle, delegate_noop};
use wayland_protocols::ext::idle_notify::v1::client::ext_idle_notification_v1::{
    self, ExtIdleNotificationV1,
};
use wayland_protocols::ext::idle_notify::v1::client::ext_idle_notifier_v1::ExtIdleNotifierV1;
use wayland_protocols::wp::idle_inhibit::zv1::client::zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1;
use wayland_protocols::wp::idle_inhibit::zv1::client::zwp_idle_inhibitor_v1::ZwpIdleInhibitorV1;

use crate::client::{Client, ToplevelId};
use crate::error::ClientError;
use crate::state::ClientState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IdleWatchId(usize);

delegate_noop!(ClientState: ExtIdleNotifierV1);
delegate_noop!(ClientState: ZwpIdleInhibitManagerV1);
delegate_noop!(ClientState: ZwpIdleInhibitorV1);

impl Dispatch<ExtIdleNotificationV1, usize> for ClientState {
    fn event(
        state: &mut Self,
        _notification: &ExtIdleNotificationV1,
        event: ext_idle_notification_v1::Event,
        watch: &usize,
        _connection: &Connection,
        _queue: &QueueHandle<Self>,
    ) {
        let Some(idle) = state.idle_watches.get_mut(*watch) else {
            return;
        };
        match event {
            ext_idle_notification_v1::Event::Idled => *idle = true,
            ext_idle_notification_v1::Event::Resumed => *idle = false,
            _ => {}
        }
    }
}

impl Client {
    pub fn watch_idleness(&mut self, timeout: Duration) -> Result<IdleWatchId, ClientError> {
        let notifier: ExtIdleNotifierV1 = self.bind(1..=2, "ext_idle_notifier_v1")?;
        let watch = self.state.idle_watches.len();
        let handle = self.queue.handle();
        notifier.get_idle_notification(milliseconds(timeout), &self.seat, &handle, watch);
        self.state.idle_watches.push(false);
        self.roundtrip()?;
        Ok(IdleWatchId(watch))
    }

    pub fn watch_input_idleness(&mut self, timeout: Duration) -> Result<IdleWatchId, ClientError> {
        let notifier: ExtIdleNotifierV1 = self.bind(2..=2, "ext_idle_notifier_v1")?;
        let watch = self.state.idle_watches.len();
        let handle = self.queue.handle();
        notifier.get_input_idle_notification(milliseconds(timeout), &self.seat, &handle, watch);
        self.state.idle_watches.push(false);
        self.roundtrip()?;
        Ok(IdleWatchId(watch))
    }

    pub fn inhibit_idleness(&mut self, toplevel: ToplevelId) -> Result<(), ClientError> {
        let manager: ZwpIdleInhibitManagerV1 = self.bind(1..=1, "zwp_idle_inhibit_manager_v1")?;
        let surface = self.surface_of(toplevel)?;
        let inhibitor = manager.create_inhibitor(surface, &self.queue.handle(), ());
        self.state.idle_inhibitors.push(inhibitor);
        self.roundtrip()
    }

    pub fn allow_idleness(&mut self) -> Result<(), ClientError> {
        for inhibitor in self.state.idle_inhibitors.drain(..) {
            inhibitor.destroy();
        }
        self.roundtrip()
    }

    #[must_use]
    pub fn is_idle(&self, watch: IdleWatchId) -> bool {
        self.state
            .idle_watches
            .get(watch.0)
            .copied()
            .unwrap_or(false)
    }
}

fn milliseconds(timeout: Duration) -> u32 {
    u32::try_from(timeout.as_millis()).unwrap_or(u32::MAX)
}

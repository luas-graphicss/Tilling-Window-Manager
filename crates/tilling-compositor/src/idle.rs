mod tracker;

use std::time::Duration;

use smithay::reexports::calloop::timer::{TimeoutAction, Timer};
use smithay::reexports::wayland_protocols::ext::idle_notify::v1::server::ext_idle_notification_v1::ExtIdleNotificationV1;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;

use crate::protocol::root_of;
use crate::runtime::Compositor;
pub(crate) use tracker::{IdleTracker, InhibitorPolicy};

impl Compositor {
    pub(crate) fn note_activity(&mut self) {
        let now = self.elapsed();
        self.idle.resume(now);
        if !self.idle.is_timed() {
            self.schedule_idleness();
        }
    }

    pub(crate) fn watch_idleness(
        &mut self,
        notification: ExtIdleNotificationV1,
        timeout_ms: u32,
        policy: InhibitorPolicy,
    ) {
        let timeout = Duration::from_millis(u64::from(timeout_ms));
        self.idle.watch(notification, timeout, policy);
        self.schedule_idleness();
    }

    pub(crate) fn release_idle_inhibitor(&mut self, surface: &WlSurface) {
        let now = self.elapsed();
        self.idle.uninhibit(surface, now);
        self.schedule_idleness();
    }

    pub(crate) fn idle_inhibited(&self) -> bool {
        !self.lock.is_engaged()
            && self
                .idle
                .inhibitors()
                .any(|surface| self.is_shown(&root_of(surface)))
    }

    fn is_shown(&self, root: &WlSurface) -> bool {
        let window_shown = self
            .windows
            .id_of(root)
            .is_some_and(|window| self.scene.window(window).is_some());
        window_shown || self.layer_on_output(root).is_some()
    }

    fn schedule_idleness(&mut self) {
        if let Some(timer) = self.idle.take_timer() {
            self.loop_handle.remove(timer);
        }
        let Some(delay) = self.next_idle_check() else {
            return;
        };
        let timer = Timer::from_duration(delay);
        let armed = self.loop_handle.insert_source(timer, |_, (), runtime| {
            let compositor = &mut runtime.compositor;
            match compositor.next_idle_check() {
                Some(delay) => TimeoutAction::ToDuration(delay),
                None => {
                    compositor.idle.take_timer();
                    TimeoutAction::Drop
                }
            }
        });
        match armed {
            Ok(timer) => self.idle.set_timer(timer),
            Err(error) => tracing::warn!(error = %error.error, "arm the idle timer"),
        }
    }

    fn next_idle_check(&mut self) -> Option<Duration> {
        let inhibited = self.idle_inhibited();
        let now = self.elapsed();
        self.idle.advance(now, inhibited)
    }
}

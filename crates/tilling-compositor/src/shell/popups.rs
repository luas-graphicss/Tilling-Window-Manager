use smithay::desktop::PopupKind;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::shell::xdg::{PopupSurface, PositionerState};

use crate::runtime::Compositor;

impl Compositor {
    pub(crate) fn track_popup(&mut self, popup: PopupSurface, positioner: PositionerState) {
        popup.with_pending_state(|state| {
            state.geometry = positioner.get_geometry();
            state.positioner = positioner;
        });
        if let Err(error) = self.popups.track_popup(PopupKind::Xdg(popup)) {
            tracing::warn!(%error, "track popup");
        }
    }

    pub(crate) fn popup_committed(&mut self, surface: &WlSurface) {
        self.popups.commit(surface);
        let Some(PopupKind::Xdg(popup)) = self.popups.find_popup(surface) else {
            return;
        };
        if popup.is_initial_configure_sent() {
            return;
        }
        if let Err(error) = popup.send_configure() {
            tracing::warn!(%error, "configure popup");
        }
    }
}

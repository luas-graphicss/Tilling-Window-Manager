use smithay::desktop::{PopupKind, PopupManager};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Logical, Rectangle};
use smithay::wayland::input_method::{InputMethodHandler, PopupSurface};

use crate::runtime::Compositor;

impl InputMethodHandler for Compositor {
    fn new_popup(&mut self, surface: PopupSurface) {
        if let Err(error) = self.popups.track_popup(PopupKind::from(surface)) {
            tracing::warn!(%error, "track input method popup");
        }
    }

    fn dismiss_popup(&mut self, surface: PopupSurface) {
        let Some(parent) = surface.get_parent().map(|parent| parent.surface.clone()) else {
            return;
        };
        if let Err(error) = PopupManager::dismiss_popup(&parent, &PopupKind::from(surface)) {
            tracing::debug!(%error, "dismiss input method popup");
        }
    }

    fn popup_repositioned(&mut self, _surface: PopupSurface) {
        self.demand_frame();
    }

    fn parent_geometry(&self, parent: &WlSurface) -> Rectangle<i32, Logical> {
        self.windows
            .get(parent)
            .map(|managed| managed.window.geometry())
            .unwrap_or_default()
    }
}

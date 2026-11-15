use smithay::reexports::wayland_server::Resource;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::SERIAL_COUNTER;
use smithay::wayland::selection::data_device::set_data_device_focus;
use smithay::wayland::selection::primary_selection::set_primary_focus;

use crate::protocol::root_of;
use crate::runtime::Compositor;

impl Compositor {
    pub(crate) fn sync_keyboard_focus(&mut self) {
        let Some(keyboard) = self.seat.get_keyboard() else {
            return;
        };
        let target = self.keyboard_target();
        keyboard.set_focus(self, target, SERIAL_COUNTER.next_serial());
    }

    pub(crate) fn refresh_pointer_focus(&mut self) {
        let Some(pointer) = self.seat.get_pointer() else {
            return;
        };
        self.handle_pointer_motion(pointer.current_location(), self.event_time());
    }

    pub(crate) fn offer_selections_to(&self, focused: Option<&WlSurface>) {
        let client = focused.and_then(|surface| self.display_handle.get_client(surface.id()).ok());
        set_data_device_focus(&self.display_handle, &self.seat, client.clone());
        set_primary_focus(&self.display_handle, &self.seat, client);
    }

    pub(crate) fn drop_selections_of_departed_clients(&self) {
        if !self.departures.take() {
            return;
        }
        let focused = self
            .seat
            .get_keyboard()
            .and_then(|keyboard| keyboard.current_focus());
        self.offer_selections_to(None);
        self.offer_selections_to(focused.as_ref());
    }

    pub(crate) fn owns_keyboard_focus(&self, surface: &WlSurface) -> bool {
        self.seat
            .get_keyboard()
            .and_then(|keyboard| keyboard.current_focus())
            .is_some_and(|focused| root_of(&focused) == root_of(surface))
    }

    fn keyboard_target(&self) -> Option<WlSurface> {
        if self.lock.is_engaged() {
            return self.lock_keyboard_target();
        }
        self.layer_keyboard_target().or_else(|| {
            let window = self.desktop.focused_window()?;
            self.windows.surface_of(window).cloned()
        })
    }
}

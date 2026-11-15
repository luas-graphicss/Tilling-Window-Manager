use smithay::input::Seat;
use smithay::input::pointer::{Focus, GrabStartData, PointerHandle};
use smithay::reexports::wayland_server::Resource;
use smithay::reexports::wayland_server::protocol::wl_seat::WlSeat;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::Serial;
use smithay::wayland::shell::xdg::ToplevelSurface;
use tilling_core::WindowMode;

use super::drag::Drag;
use crate::input::DragGrab;
use crate::runtime::Compositor;

impl Compositor {
    pub(crate) fn begin_drag(
        &mut self,
        toplevel: &ToplevelSurface,
        seat: &WlSeat,
        serial: Serial,
        drag: Drag,
    ) {
        let Some(pointer) = Seat::<Self>::from_resource(seat).and_then(|seat| seat.get_pointer())
        else {
            return;
        };
        let surface = toplevel.wl_surface();
        let Some(start_data) = authorizing_press(&pointer, surface, serial) else {
            return;
        };
        let Some(window) = self.windows.id_of(surface) else {
            return;
        };
        let mode = self.desktop.window(window).map(|state| state.mode());
        let Some(WindowMode::Floating(initial)) = mode else {
            return;
        };
        let grab = DragGrab::new(start_data, window, initial, drag);
        pointer.set_grab(self, grab, serial, Focus::Clear);
    }
}

fn authorizing_press(
    pointer: &PointerHandle<Compositor>,
    surface: &WlSurface,
    serial: Serial,
) -> Option<GrabStartData<Compositor>> {
    if !pointer.has_grab(serial) {
        return None;
    }
    let start_data = pointer.grab_start_data()?;
    let (pressed, _) = start_data.focus.as_ref()?;
    pressed
        .id()
        .same_client_as(&surface.id())
        .then_some(start_data)
}

use smithay::desktop::{
    PopupGrab, PopupKeyboardGrab, PopupKind, PopupPointerGrab, PopupUngrabStrategy,
    find_popup_root_surface, get_popup_toplevel_coords,
};
use smithay::input::Seat;
use smithay::input::pointer::Focus;
use smithay::reexports::wayland_server::Resource;
use smithay::reexports::wayland_server::protocol::wl_seat::WlSeat;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Logical, Rectangle, Serial};
use smithay::wayland::shell::xdg::{PopupSurface, PositionerState};

use crate::geometry::rectangle;
use crate::runtime::Compositor;

impl Compositor {
    pub(crate) fn track_popup(&mut self, popup: PopupSurface, positioner: PositionerState) {
        self.place_popup(&popup, positioner);
        if let Err(error) = self.popups.track_popup(PopupKind::Xdg(popup)) {
            tracing::warn!(%error, "track popup");
        }
    }

    pub(crate) fn place_popup(&self, popup: &PopupSurface, positioner: PositionerState) {
        let geometry = match self.popup_bounds(popup) {
            Some(bounds) => positioner.get_unconstrained_geometry(bounds),
            None => positioner.get_geometry(),
        };
        popup.with_pending_state(|state| {
            state.geometry = geometry;
            state.positioner = positioner;
        });
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

    pub(crate) fn grab_popup(&mut self, popup: PopupSurface, seat: &WlSeat, serial: Serial) {
        let Some(seat) = Seat::<Self>::from_resource(seat) else {
            return;
        };
        let kind = PopupKind::Xdg(popup.clone());
        let authorized_root = find_popup_root_surface(&kind)
            .ok()
            .filter(|root| self.may_grab(root, serial));
        let Some(root) = authorized_root else {
            popup.send_popup_done();
            return;
        };
        match self.popups.grab_popup(root, kind, &seat, serial) {
            Ok(grab) => self.install_popup_grab(&seat, grab, serial),
            Err(error) => {
                tracing::debug!(%error, "refuse popup grab");
                popup.send_popup_done();
            }
        }
    }

    fn install_popup_grab(&mut self, seat: &Seat<Self>, mut grab: PopupGrab<Self>, serial: Serial) {
        let previous = grab.previous_serial().unwrap_or(serial);
        if let Some(keyboard) = seat.get_keyboard() {
            let foreign = !keyboard.has_grab(serial) && !keyboard.has_grab(previous);
            if keyboard.is_grabbed() && foreign {
                grab.ungrab(PopupUngrabStrategy::All);
                return;
            }
            keyboard.set_focus(self, grab.current_grab(), serial);
            keyboard.set_grab(self, PopupKeyboardGrab::new(&grab), serial);
        }
        if let Some(pointer) = seat.get_pointer() {
            let foreign = !pointer.has_grab(serial) && !pointer.has_grab(previous);
            if pointer.is_grabbed() && foreign {
                grab.ungrab(PopupUngrabStrategy::All);
                return;
            }
            pointer.set_grab(self, PopupPointerGrab::new(&grab), serial, Focus::Keep);
        }
    }

    fn may_grab(&self, root: &WlSurface, serial: Serial) -> bool {
        let keyboard = self
            .seat
            .get_keyboard()
            .and_then(|keyboard| keyboard.current_focus());
        let pointer = self
            .seat
            .get_pointer()
            .and_then(|pointer| pointer.current_focus());
        let owns_focus = [keyboard, pointer]
            .into_iter()
            .flatten()
            .any(|focused| focused.id().same_client_as(&root.id()));
        let managed = self.windows.get(root).is_some() || self.layer_on_output(root).is_some();
        owns_focus && managed && self.input_serials.contains(serial)
    }

    fn popup_bounds(&self, popup: &PopupSurface) -> Option<Rectangle<i32, Logical>> {
        let kind = PopupKind::Xdg(popup.clone());
        let root = find_popup_root_surface(&kind).ok()?;
        let placed = self.scene.window(self.windows.id_of(&root)?)?;
        let output = self.desktop.output(placed.output)?;
        let mut bounds = rectangle(output.area());
        bounds.loc -= rectangle(placed.content).loc + get_popup_toplevel_coords(&kind);
        Some(bounds)
    }
}

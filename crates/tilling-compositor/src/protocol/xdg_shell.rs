use smithay::desktop::Window;
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::ResizeEdge;
use smithay::reexports::wayland_server::protocol::wl_output::WlOutput;
use smithay::reexports::wayland_server::protocol::wl_seat::WlSeat;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::Serial;
use smithay::wayland::shell::xdg::{
    Configure, PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
};

use crate::runtime::Compositor;
use crate::shell::{Drag, Edges};
use crate::transaction::ConfigureSerial;

impl XdgShellHandler for Compositor {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        self.windows
            .insert(Window::new_wayland_window(surface.clone()), surface);
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        self.forget_toplevel(surface.wl_surface());
    }

    fn new_popup(&mut self, surface: PopupSurface, positioner: PositionerState) {
        self.track_popup(surface, positioner);
    }

    fn move_request(&mut self, surface: ToplevelSurface, seat: WlSeat, serial: Serial) {
        self.begin_drag(&surface, &seat, serial, Drag::Move);
    }

    fn resize_request(
        &mut self,
        surface: ToplevelSurface,
        seat: WlSeat,
        serial: Serial,
        edges: ResizeEdge,
    ) {
        if let Some(edges) = dragged_edges(edges) {
            self.begin_drag(&surface, &seat, serial, Drag::Resize(edges));
        }
    }

    fn grab(&mut self, _surface: PopupSurface, _seat: WlSeat, _serial: Serial) {}

    fn reposition_request(
        &mut self,
        surface: PopupSurface,
        positioner: PositionerState,
        token: u32,
    ) {
        surface.with_pending_state(|state| {
            state.geometry = positioner.get_geometry();
            state.positioner = positioner;
        });
        surface.send_repositioned(token);
    }

    fn ack_configure(&mut self, surface: WlSurface, configure: Configure) {
        let Configure::Toplevel(configure) = configure else {
            return;
        };
        if let Some(window) = self.windows.id_of(&surface) {
            let serial = ConfigureSerial(u32::from(configure.serial));
            self.transactions.acknowledge(window, serial);
        }
    }

    fn fullscreen_request(&mut self, surface: ToplevelSurface, _output: Option<WlOutput>) {
        self.set_fullscreen(&surface, true);
    }

    fn unfullscreen_request(&mut self, surface: ToplevelSurface) {
        self.set_fullscreen(&surface, false);
    }

    fn app_id_changed(&mut self, surface: ToplevelSurface) {
        self.refresh_metadata(surface.wl_surface());
    }

    fn title_changed(&mut self, surface: ToplevelSurface) {
        self.refresh_metadata(surface.wl_surface());
    }
}

fn dragged_edges(edges: ResizeEdge) -> Option<Edges> {
    let (left, right, top, bottom) = match edges {
        ResizeEdge::Top => (false, false, true, false),
        ResizeEdge::Bottom => (false, false, false, true),
        ResizeEdge::Left => (true, false, false, false),
        ResizeEdge::TopLeft => (true, false, true, false),
        ResizeEdge::BottomLeft => (true, false, false, true),
        ResizeEdge::Right => (false, true, false, false),
        ResizeEdge::TopRight => (false, true, true, false),
        ResizeEdge::BottomRight => (false, true, false, true),
        _ => return None,
    };
    Some(Edges {
        left,
        right,
        top,
        bottom,
    })
}

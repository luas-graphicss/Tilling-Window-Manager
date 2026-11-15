use smithay::output::Output;
use smithay::reexports::wayland_server::protocol::wl_output::WlOutput;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Logical, Point, SERIAL_COUNTER, Size};
use smithay::wayland::session_lock::{LockSurface, SessionLocker};

use crate::geometry::logical_size;
use crate::runtime::Compositor;
use crate::shell::SurfaceHit;

impl Compositor {
    pub(crate) fn lock_for_client(&mut self, confirmation: SessionLocker) {
        if self.lock.is_held() {
            return;
        }
        self.lock.hold(confirmation);
        self.lock_session();
    }

    pub(crate) fn lock_session(&mut self) {
        let already_engaged = self.lock.is_engaged();
        let outputs = self.outputs.iter().map(|(_, output)| output.clone());
        self.lock.engage(outputs.collect());
        if !already_engaged {
            self.withdraw_input_from_clients();
        }
        self.sync_keyboard_focus();
        self.demand_frame();
    }

    pub(crate) fn unlock_session(&mut self) {
        self.lock.release();
        self.sync_keyboard_focus();
        self.refresh_pointer_focus();
        self.demand_frame();
    }

    pub(crate) fn admit_lock_surface(&mut self, surface: LockSurface, output: &WlOutput) {
        let Some(output) = Output::from_resource(output) else {
            return;
        };
        configure_lock_surface(&surface, &output);
        self.lock.show(output, surface);
        self.sync_keyboard_focus();
        self.demand_frame();
    }

    pub(crate) fn resize_lock_surface(&self, output: &Output) {
        if let Some(surface) = self.lock.surface_on(output) {
            configure_lock_surface(surface, output);
        }
    }

    pub(crate) fn lock_keyboard_target(&self) -> Option<WlSurface> {
        let preferred = self.active_output();
        let on_preferred = preferred.and_then(|output| self.lock.surface_on(output));
        on_preferred
            .or_else(|| self.lock.surfaces().next().map(|(_, surface)| surface))
            .map(|surface| surface.wl_surface().clone())
    }

    pub(crate) fn lock_surface_under(&self, location: Point<f64, Logical>) -> Option<SurfaceHit> {
        let output = self.output_at(location)?;
        let surface = self.lock.surface_on(output)?;
        let origin = output.current_location().to_f64();
        Some((surface.wl_surface().clone(), origin))
    }

    fn withdraw_input_from_clients(&mut self) {
        let serial = SERIAL_COUNTER.next_serial();
        let time = self.event_time();
        if let Some(keyboard) = self.seat.get_keyboard() {
            keyboard.unset_grab(self);
        }
        if let Some(pointer) = self.seat.get_pointer() {
            pointer.unset_grab(self, serial, time);
        }
        self.drag_icon = None;
        self.layer_focus = None;
        self.refresh_pointer_focus();
    }
}

fn configure_lock_surface(surface: &LockSurface, output: &Output) {
    let Some(size) = covering_size(output) else {
        return;
    };
    surface.with_pending_state(|state| state.size = Some(size));
    surface.send_configure();
}

fn covering_size(output: &Output) -> Option<Size<u32, Logical>> {
    let logical = logical_size(output)?;
    let width = u32::try_from(logical.w).ok()?;
    let height = u32::try_from(logical.h).ok()?;
    Some(Size::from((width, height)))
}

use smithay::backend::input::ButtonState;
use smithay::input::pointer::{ButtonEvent, MotionEvent};
use smithay::utils::{Logical, Point, SERIAL_COUNTER};
use tilling_core::Command;

use crate::geometry::nearest_inside;
use crate::runtime::Compositor;

impl Compositor {
    pub(crate) fn confine_pointer(&mut self) {
        let Some(pointer) = self.seat.get_pointer() else {
            return;
        };
        let location = pointer.current_location();
        let areas = self.desktop.outputs().map(|(_, output)| output.area());
        let Some(confined) = nearest_inside(areas, location) else {
            return;
        };
        if confined != location {
            self.handle_pointer_motion(confined, self.event_time());
        }
    }

    pub(crate) fn handle_pointer_motion(&mut self, location: Point<f64, Logical>, time: u32) {
        let Some(pointer) = self.seat.get_pointer() else {
            return;
        };
        let focus = self.surface_under(location);
        let event = MotionEvent {
            location,
            serial: SERIAL_COUNTER.next_serial(),
            time,
        };
        pointer.motion(self, focus, &event);
        pointer.frame(self);
        if self.draws_named_cursor || self.cursor_surface().is_some() {
            self.demand_frame();
        }
    }

    #[cfg(any(feature = "nested", feature = "drm"))]
    pub(crate) fn handle_pointer_delta(&mut self, delta: Point<f64, Logical>, time: u32) {
        let Some(pointer) = self.seat.get_pointer() else {
            return;
        };
        let wanted = pointer.current_location() + delta;
        let areas = self.desktop.outputs().map(|(_, output)| output.area());
        if let Some(location) = nearest_inside(areas, wanted) {
            self.handle_pointer_motion(location, time);
        }
    }

    #[cfg(feature = "drm")]
    pub(crate) fn cancel_pointer_grab(&mut self) {
        let Some(pointer) = self.seat.get_pointer() else {
            return;
        };
        if pointer.is_grabbed() {
            pointer.unset_grab(self, SERIAL_COUNTER.next_serial(), self.event_time());
        }
    }

    pub(crate) fn handle_pointer_button(&mut self, button: u32, state: ButtonState, time: u32) {
        let Some(pointer) = self.seat.get_pointer() else {
            return;
        };
        if state == ButtonState::Pressed && !pointer.is_grabbed() {
            self.focus_window_at(pointer.current_location());
        }
        let event = ButtonEvent {
            serial: SERIAL_COUNTER.next_serial(),
            time,
            button,
            state,
        };
        self.input_serials.record(event.serial);
        pointer.button(self, &event);
        pointer.frame(self);
    }

    fn focus_window_at(&mut self, location: Point<f64, Logical>) {
        let Some(window) = self.window_at(location) else {
            return;
        };
        if self.desktop.focused_window() != Some(window) {
            self.run(Command::FocusWindow { window });
        }
    }
}

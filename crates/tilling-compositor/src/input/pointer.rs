use smithay::backend::input::ButtonState;
use smithay::input::pointer::{ButtonEvent, MotionEvent, RelativeMotionEvent};
use smithay::utils::{Logical, Point, SERIAL_COUNTER};
use tilling_core::Command;

use crate::geometry::nearest_inside;
use crate::protocol::root_of;
use crate::runtime::Compositor;

const MICROSECONDS_PER_MILLISECOND: u64 = 1000;

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
        self.engage_pointer_constraint();
        let pointer_is_drawn = self.draws_named_cursor || self.cursor_surface().is_some();
        if pointer_is_drawn || self.drag_icon.is_some() {
            self.demand_frame();
        }
    }

    pub(crate) fn handle_pointer_delta(&mut self, delta: Point<f64, Logical>, time: u32) {
        let Some(pointer) = self.seat.get_pointer() else {
            return;
        };
        let location = pointer.current_location();
        let focus = self.surface_under(location);
        let constraint = self.held_constraint(&pointer, focus.as_ref(), location);
        let event = RelativeMotionEvent {
            delta,
            delta_unaccel: delta,
            utime: u64::from(time) * MICROSECONDS_PER_MILLISECOND,
        };
        pointer.relative_motion(self, focus.clone(), &event);
        let areas = self.desktop.outputs().map(|(_, output)| output.area());
        let target = nearest_inside(areas, location + delta)
            .filter(|target| self.constraint_allows(constraint.as_ref(), focus.as_ref(), *target));
        match target {
            Some(target) => self.handle_pointer_motion(target, time),
            None => pointer.frame(self),
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
        let chooses_focus = state == ButtonState::Pressed && !pointer.is_grabbed();
        if chooses_focus && !self.lock.is_engaged() {
            self.focus_at(pointer.current_location());
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

    fn focus_at(&mut self, location: Point<f64, Logical>) {
        let pressed = self
            .surface_under(location)
            .map(|(surface, _)| root_of(&surface));
        let on_layer = pressed
            .as_ref()
            .is_some_and(|surface| self.layer_on_output(surface).is_some());
        self.choose_layer_at_press(pressed.as_ref());
        if !on_layer {
            self.focus_window_at(location);
        }
        self.sync_keyboard_focus();
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

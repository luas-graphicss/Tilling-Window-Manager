use smithay::backend::input::ButtonState;
use smithay::input::pointer::{ButtonEvent, MotionEvent};
use smithay::utils::{Logical, Point, SERIAL_COUNTER};
use tilling_core::Command;

use crate::runtime::Compositor;

impl Compositor {
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

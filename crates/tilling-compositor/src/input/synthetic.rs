use smithay::backend::input::{AxisSource, ButtonState, KeyState};
use smithay::input::keyboard::Keycode;
use smithay::utils::Point;

use super::scroll::Scroll;
use crate::runtime::Compositor;

const XKB_KEYCODE_OFFSET: u32 = 8;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SyntheticInput {
    Key { code: u32, pressed: bool },
    PointerMotion { x: f64, y: f64 },
    PointerButton { code: u32, pressed: bool },
    PointerScroll { horizontal: f64, vertical: f64 },
    HostFocusLost,
}

impl Compositor {
    pub(crate) fn inject(&mut self, input: SyntheticInput) {
        let time = self.event_time();
        match input {
            SyntheticInput::Key { code, pressed } => {
                let Some(keycode) = code.checked_add(XKB_KEYCODE_OFFSET) else {
                    return;
                };
                let state = if pressed {
                    KeyState::Pressed
                } else {
                    KeyState::Released
                };
                self.handle_key(Keycode::new(keycode), state, time);
            }
            SyntheticInput::PointerMotion { x, y } => {
                self.handle_pointer_motion(Point::from((x, y)), time);
            }
            SyntheticInput::PointerButton { code, pressed } => {
                let state = if pressed {
                    ButtonState::Pressed
                } else {
                    ButtonState::Released
                };
                self.handle_pointer_button(code, state, time);
            }
            SyntheticInput::PointerScroll {
                horizontal,
                vertical,
            } => {
                let scroll = Scroll {
                    source: AxisSource::Wheel,
                    horizontal,
                    vertical,
                };
                self.handle_pointer_scroll(scroll, time);
            }
            SyntheticInput::HostFocusLost => self.release_held_keys(),
        }
    }
}

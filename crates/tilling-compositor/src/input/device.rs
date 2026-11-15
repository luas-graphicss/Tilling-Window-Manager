use smithay::backend::input::{
    AbsolutePositionEvent, Axis, Event, InputBackend, InputEvent, KeyboardKeyEvent,
    PointerAxisEvent, PointerButtonEvent,
};
use smithay::output::Output;
use smithay::utils::{Logical, Size};

use super::scroll::Scroll;
use crate::runtime::Compositor;

const WHEEL_STEP: f64 = 15.0;
const HIGH_RESOLUTION_STEP: f64 = 120.0;

impl Compositor {
    pub(crate) fn handle_device_event<B: InputBackend>(
        &mut self,
        event: InputEvent<B>,
        output: &Output,
    ) {
        match event {
            InputEvent::Keyboard { event } => {
                self.handle_key(event.key_code(), event.state(), event.time_msec());
            }
            InputEvent::PointerMotionAbsolute { event } => {
                let Some(area) = logical_size(output) else {
                    return;
                };
                let location =
                    event.position_transformed(area) + output.current_location().to_f64();
                self.handle_pointer_motion(location, event.time_msec());
            }
            InputEvent::PointerButton { event } => {
                self.handle_pointer_button(event.button_code(), event.state(), event.time_msec());
            }
            InputEvent::PointerAxis { event } => {
                self.handle_pointer_scroll(scroll_of::<B>(&event), event.time_msec());
            }
            _ => {}
        }
    }
}

fn logical_size(output: &Output) -> Option<Size<i32, Logical>> {
    let scale = output.current_scale().fractional_scale();
    let mode = output.current_mode()?;
    Some(mode.size.to_f64().to_logical(scale).to_i32_round())
}

fn scroll_of<B: InputBackend>(event: &B::PointerAxisEvent) -> Scroll {
    let amount = |axis| {
        let stepped = || {
            event
                .amount_v120(axis)
                .map(|steps| steps * WHEEL_STEP / HIGH_RESOLUTION_STEP)
        };
        event.amount(axis).or_else(stepped).unwrap_or(0.0)
    };
    Scroll {
        source: event.source(),
        horizontal: amount(Axis::Horizontal),
        vertical: amount(Axis::Vertical),
    }
}

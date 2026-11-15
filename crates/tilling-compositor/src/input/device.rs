use smithay::backend::input::{
    AbsolutePositionEvent, Axis, Event, InputBackend, InputEvent, KeyboardKeyEvent,
    PointerAxisEvent, PointerButtonEvent, PointerMotionEvent,
};
use smithay::output::Output;

use super::scroll::Scroll;
use crate::geometry::logical_size;
use crate::runtime::Compositor;

const WHEEL_STEP: f64 = 15.0;
const HIGH_RESOLUTION_STEP: f64 = 120.0;

impl Compositor {
    pub(crate) fn handle_device_event<B: InputBackend>(
        &mut self,
        event: InputEvent<B>,
        absolute_target: Option<&Output>,
    ) {
        self.note_activity();
        match event {
            InputEvent::Keyboard { event } => {
                self.handle_key(event.key_code(), event.state(), event.time_msec());
            }
            InputEvent::PointerMotion { event } => {
                self.handle_pointer_delta(event.delta(), event.time_msec());
            }
            InputEvent::PointerMotionAbsolute { event } => {
                let Some(output) = absolute_target else {
                    return;
                };
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

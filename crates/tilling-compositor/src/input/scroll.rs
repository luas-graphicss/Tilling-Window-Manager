use smithay::backend::input::{Axis, AxisSource};
use smithay::input::pointer::AxisFrame;

use crate::runtime::Compositor;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Scroll {
    pub(crate) source: AxisSource,
    pub(crate) horizontal: f64,
    pub(crate) vertical: f64,
}

impl Scroll {
    fn frame(self, time: u32, natural: bool) -> AxisFrame {
        let direction = if natural { -1.0 } else { 1.0 };
        let axes = [
            (Axis::Horizontal, self.horizontal),
            (Axis::Vertical, self.vertical),
        ];
        let mut frame = AxisFrame::new(time).source(self.source);
        for (axis, amount) in axes {
            if amount != 0.0 {
                frame = frame.value(axis, amount * direction);
            }
        }
        frame
    }
}

impl Compositor {
    pub(crate) fn handle_pointer_scroll(&mut self, scroll: Scroll, time: u32) {
        let Some(pointer) = self.seat.get_pointer() else {
            return;
        };
        let natural = self.config.input().pointer.natural_scroll;
        pointer.axis(self, scroll.frame(time, natural));
        pointer.frame(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHEEL: Scroll = Scroll {
        source: AxisSource::Wheel,
        horizontal: 0.0,
        vertical: 15.0,
    };

    #[test]
    fn only_the_axes_that_moved_are_reported() {
        let frame = WHEEL.frame(40, false);

        assert_eq!(frame.axis, (0.0, 15.0));
        assert_eq!(frame.source, Some(AxisSource::Wheel));
        assert_eq!(frame.time, 40);
    }

    #[test]
    fn natural_scrolling_inverts_the_reported_amount() {
        let frame = WHEEL.frame(40, true);

        assert_eq!(frame.axis, (0.0, -15.0));
    }
}

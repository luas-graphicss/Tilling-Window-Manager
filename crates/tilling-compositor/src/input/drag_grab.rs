use smithay::input::pointer::{
    AxisFrame, ButtonEvent, GestureHoldBeginEvent, GestureHoldEndEvent, GesturePinchBeginEvent,
    GesturePinchEndEvent, GesturePinchUpdateEvent, GestureSwipeBeginEvent, GestureSwipeEndEvent,
    GestureSwipeUpdateEvent, GrabStartData, MotionEvent, PointerGrab, PointerInnerHandle,
    RelativeMotionEvent,
};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Logical, Point};
use tilling_core::geometry::LogicalRect;
use tilling_core::{Command, ModeChange, WindowId, WindowMode};

use crate::runtime::Compositor;
use crate::shell::Drag;

type Handle<'a, 'b> = &'a mut PointerInnerHandle<'b, Compositor>;
type Focus = Option<(WlSurface, Point<f64, Logical>)>;

pub(crate) struct DragGrab {
    start_data: GrabStartData<Compositor>,
    window: WindowId,
    initial: LogicalRect,
    drag: Drag,
}

impl DragGrab {
    pub(crate) fn new(
        start_data: GrabStartData<Compositor>,
        window: WindowId,
        initial: LogicalRect,
        drag: Drag,
    ) -> Self {
        Self {
            start_data,
            window,
            initial,
            drag,
        }
    }

    fn follow(&self, compositor: &mut Compositor, location: Point<f64, Logical>) {
        let mode = compositor
            .desktop
            .window(self.window)
            .map(|state| state.mode());
        if !matches!(mode, Some(WindowMode::Floating(_))) {
            return;
        }
        let travelled: Point<i32, Logical> = (location - self.start_data.location).to_i32_round();
        let Some(area) = self.drag.apply(self.initial, travelled.x, travelled.y) else {
            return;
        };
        compositor.run(Command::ChangeWindowMode {
            window: self.window,
            change: ModeChange::Float(area),
        });
    }
}

impl PointerGrab<Compositor> for DragGrab {
    fn motion(
        &mut self,
        data: &mut Compositor,
        handle: Handle<'_, '_>,
        _focus: Focus,
        event: &MotionEvent,
    ) {
        handle.motion(data, None, event);
        self.follow(data, event.location);
    }

    fn relative_motion(
        &mut self,
        data: &mut Compositor,
        handle: Handle<'_, '_>,
        _focus: Focus,
        event: &RelativeMotionEvent,
    ) {
        handle.relative_motion(data, None, event);
    }

    fn button(&mut self, data: &mut Compositor, handle: Handle<'_, '_>, event: &ButtonEvent) {
        handle.button(data, event);
        if handle.current_pressed().is_empty() {
            handle.unset_grab(self, data, event.serial, event.time, true);
        }
    }

    fn axis(&mut self, data: &mut Compositor, handle: Handle<'_, '_>, details: AxisFrame) {
        handle.axis(data, details);
    }

    fn frame(&mut self, data: &mut Compositor, handle: Handle<'_, '_>) {
        handle.frame(data);
    }

    fn gesture_swipe_begin(
        &mut self,
        data: &mut Compositor,
        handle: Handle<'_, '_>,
        event: &GestureSwipeBeginEvent,
    ) {
        handle.gesture_swipe_begin(data, event);
    }

    fn gesture_swipe_update(
        &mut self,
        data: &mut Compositor,
        handle: Handle<'_, '_>,
        event: &GestureSwipeUpdateEvent,
    ) {
        handle.gesture_swipe_update(data, event);
    }

    fn gesture_swipe_end(
        &mut self,
        data: &mut Compositor,
        handle: Handle<'_, '_>,
        event: &GestureSwipeEndEvent,
    ) {
        handle.gesture_swipe_end(data, event);
    }

    fn gesture_pinch_begin(
        &mut self,
        data: &mut Compositor,
        handle: Handle<'_, '_>,
        event: &GesturePinchBeginEvent,
    ) {
        handle.gesture_pinch_begin(data, event);
    }

    fn gesture_pinch_update(
        &mut self,
        data: &mut Compositor,
        handle: Handle<'_, '_>,
        event: &GesturePinchUpdateEvent,
    ) {
        handle.gesture_pinch_update(data, event);
    }

    fn gesture_pinch_end(
        &mut self,
        data: &mut Compositor,
        handle: Handle<'_, '_>,
        event: &GesturePinchEndEvent,
    ) {
        handle.gesture_pinch_end(data, event);
    }

    fn gesture_hold_begin(
        &mut self,
        data: &mut Compositor,
        handle: Handle<'_, '_>,
        event: &GestureHoldBeginEvent,
    ) {
        handle.gesture_hold_begin(data, event);
    }

    fn gesture_hold_end(
        &mut self,
        data: &mut Compositor,
        handle: Handle<'_, '_>,
        event: &GestureHoldEndEvent,
    ) {
        handle.gesture_hold_end(data, event);
    }

    fn start_data(&self) -> &GrabStartData<Compositor> {
        &self.start_data
    }

    fn unset(&mut self, _data: &mut Compositor) {}
}

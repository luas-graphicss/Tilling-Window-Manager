#![cfg(feature = "nested")]

use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::winit::{self, WinitEventLoop, WinitGraphicsBackend};
use smithay::reexports::calloop::EventSource;

type NestedBackend = (WinitGraphicsBackend<GlesRenderer>, WinitEventLoop);

fn assert_event_source<S: EventSource>() {}

fn assert_initializer<F: Fn() -> Result<NestedBackend, winit::Error>>(_initializer: F) {}

#[test]
fn winit_backend_renders_with_gles_and_feeds_the_event_loop() {
    assert_event_source::<WinitEventLoop>();
    assert_initializer(winit::init::<GlesRenderer>);
}

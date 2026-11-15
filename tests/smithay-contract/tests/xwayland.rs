#![cfg(feature = "xwayland")]

use smithay::reexports::calloop::EventSource;
use smithay::xwayland::{XWayland, XWaylandEvent};

fn assert_event_source<S: EventSource<Event = XWaylandEvent>>() {}

#[test]
fn xwayland_server_reports_its_lifecycle_through_the_event_loop() {
    assert_event_source::<XWayland>();
}

#![cfg(feature = "drm")]

use std::os::fd::OwnedFd;
use std::path::Path;

use smithay::backend::allocator::Allocator;
use smithay::backend::allocator::gbm::GbmAllocator;
use smithay::backend::drm::DrmDeviceFd;
use smithay::backend::libinput::LibinputInputBackend;
use smithay::backend::renderer::Renderer;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::session::Session;
use smithay::backend::session::libseat::LibSeatSession;
use smithay::backend::udev;
use smithay::reexports::calloop::EventSource;
use smithay::reexports::input::{Libinput, LibinputInterface};
use smithay::reexports::rustix::io::Errno;

struct NoDevices;

impl LibinputInterface for NoDevices {
    fn open_restricted(&mut self, _path: &Path, _flags: i32) -> Result<OwnedFd, i32> {
        Err(Errno::ACCESS.raw_os_error())
    }

    fn close_restricted(&mut self, fd: OwnedFd) {
        drop(fd);
    }
}

fn assert_event_source<S: EventSource>(_source: &S) {}

fn assert_renderer<R: Renderer>() {}

fn assert_session<S: Session>() {}

fn assert_allocator<A: Allocator>() {}

#[test]
fn udev_enumerates_gpus_without_requiring_hardware() {
    assert!(udev::all_gpus("seat0").is_ok());
}

#[test]
fn libinput_context_without_devices_feeds_the_event_loop() {
    let mut context = Libinput::new_from_path(NoDevices);

    assert!(context.dispatch().is_ok());
    assert_event_source(&LibinputInputBackend::new(context));
}

#[test]
fn session_allocator_and_renderer_satisfy_the_backend_traits() {
    assert_session::<LibSeatSession>();
    assert_allocator::<GbmAllocator<DrmDeviceFd>>();
    assert_renderer::<GlesRenderer>();
}

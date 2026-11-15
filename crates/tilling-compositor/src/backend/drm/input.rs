use smithay::backend::input::InputEvent;
use smithay::backend::libinput::{LibinputInputBackend, LibinputSessionInterface};
use smithay::backend::session::libseat::LibSeatSession;
use smithay::reexports::calloop::LoopHandle;
use smithay::reexports::input::{Device, DeviceCapability, Libinput};

use super::device_settings;
use super::seat::Seat;
use crate::error::CompositorError;
use crate::runtime::{Compositor, Runtime};

pub(super) struct InputDevices {
    context: Libinput,
    connected: Vec<Device>,
    applied_config_revision: u64,
}

impl InputDevices {
    pub(super) fn open(
        seat: &Seat,
        loop_handle: &LoopHandle<'static, Runtime>,
    ) -> Result<Self, CompositorError> {
        let opener = LibinputSessionInterface::from(seat.device_opener());
        let mut context =
            Libinput::new_with_udev::<LibinputSessionInterface<LibSeatSession>>(opener);
        context
            .udev_assign_seat(seat.name())
            .map_err(|()| CompositorError::InputSeat(seat.name().to_owned()))?;
        let events = LibinputInputBackend::new(context.clone());
        loop_handle
            .insert_source(events, |event, &mut (), runtime| {
                runtime.with_drm(|backend, compositor| backend.handle_input(event, compositor));
            })
            .map_err(|error| CompositorError::EventSource(error.error))?;
        Ok(Self {
            context,
            connected: Vec::new(),
            applied_config_revision: 0,
        })
    }

    pub(super) fn handle(
        &mut self,
        event: InputEvent<LibinputInputBackend>,
        compositor: &mut Compositor,
    ) {
        match event {
            InputEvent::DeviceAdded { mut device } => {
                device_settings::apply(&mut device, compositor.config.input());
                self.connected.push(device);
                compositor.input_devices = self.connected.len();
            }
            InputEvent::DeviceRemoved { device } => {
                self.connected.retain(|known| *known != device);
                compositor.input_devices = self.connected.len();
                self.release_lost_capabilities(compositor);
            }
            other => {
                let absolute_target = compositor
                    .outputs
                    .iter()
                    .next()
                    .map(|(_, output)| output.clone());
                compositor.handle_device_event(other, absolute_target.as_ref());
            }
        }
    }

    pub(super) fn follow_config(&mut self, compositor: &Compositor) {
        if self.applied_config_revision == compositor.config_revision {
            return;
        }
        for device in &mut self.connected {
            device_settings::apply(device, compositor.config.input());
        }
        self.applied_config_revision = compositor.config_revision;
    }

    pub(super) fn suspend(&self) {
        self.context.suspend();
    }

    pub(super) fn resume(&mut self) {
        if self.context.resume().is_err() {
            tracing::warn!("input devices did not resume");
        }
    }

    fn release_lost_capabilities(&self, compositor: &mut Compositor) {
        let remains = |capability| {
            self.connected
                .iter()
                .any(|device| device.has_capability(capability))
        };
        if !remains(DeviceCapability::Keyboard) {
            compositor.release_held_keys();
        }
        if !remains(DeviceCapability::Pointer) {
            compositor.cancel_pointer_grab();
        }
    }
}

use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Dispatch, QueueHandle, delegate_noop};
use wayland_protocols::ext::session_lock::v1::client::ext_session_lock_manager_v1::ExtSessionLockManagerV1;
use wayland_protocols::ext::session_lock::v1::client::ext_session_lock_surface_v1::{
    self, ExtSessionLockSurfaceV1,
};
use wayland_protocols::ext::session_lock::v1::client::ext_session_lock_v1::{
    self, ExtSessionLockV1,
};

use crate::buffer;
use crate::client::Client;
use crate::error::ClientError;
use crate::state::ClientState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LockSurfaceRole;

#[derive(Debug, Default)]
pub(crate) struct LockLog {
    lock: Option<ExtSessionLockV1>,
    confirmed: bool,
    refused: bool,
    configure: Option<(u32, u32, u32)>,
    covers: Vec<(WlSurface, ExtSessionLockSurfaceV1, WlBuffer)>,
}

delegate_noop!(ClientState: ExtSessionLockManagerV1);
delegate_noop!(ClientState: ignore WlOutput);

impl Dispatch<ExtSessionLockV1, ()> for ClientState {
    fn event(
        state: &mut Self,
        _lock: &ExtSessionLockV1,
        event: ext_session_lock_v1::Event,
        (): &(),
        _connection: &Connection,
        _queue: &QueueHandle<Self>,
    ) {
        match event {
            ext_session_lock_v1::Event::Locked => state.lock.confirmed = true,
            ext_session_lock_v1::Event::Finished => state.lock.refused = true,
            _ => {}
        }
    }
}

impl Dispatch<ExtSessionLockSurfaceV1, ()> for ClientState {
    fn event(
        state: &mut Self,
        _surface: &ExtSessionLockSurfaceV1,
        event: ext_session_lock_surface_v1::Event,
        (): &(),
        _connection: &Connection,
        _queue: &QueueHandle<Self>,
    ) {
        if let ext_session_lock_surface_v1::Event::Configure {
            serial,
            width,
            height,
        } = event
        {
            state.lock.configure = Some((serial, width, height));
        }
    }
}

impl Client {
    pub fn lock_session(&mut self) -> Result<(), ClientError> {
        let manager: ExtSessionLockManagerV1 = self.bind(1..=1, "ext_session_lock_manager_v1")?;
        let lock = manager.lock(&self.queue.handle(), ());
        self.state.lock = LockLog {
            lock: Some(lock),
            ..LockLog::default()
        };
        self.roundtrip()
    }

    pub fn cover_output(&mut self) -> Result<(u32, u32), ClientError> {
        let output: WlOutput = self.bind(1..=4, "wl_output")?;
        let handle = self.queue.handle();
        let surface = self.compositor.create_surface(&handle, LockSurfaceRole);
        let lock = self
            .state
            .lock
            .lock
            .as_ref()
            .ok_or(ClientError::NoSessionLock)?;
        let role = lock.get_lock_surface(&surface, &output, &handle, ());
        self.state.lock.configure = None;
        let (serial, width, height) =
            self.wait_for("a lock surface configure", |state| state.lock.configure)?;
        let buffer = buffer::create(&self.shm, &handle, width, height)?;
        role.ack_configure(serial);
        surface.attach(Some(&buffer), 0, 0);
        surface.commit();
        self.state.lock.covers.push((surface, role, buffer));
        self.roundtrip()?;
        Ok((width, height))
    }

    pub fn unlock_session(&mut self) -> Result<(), ClientError> {
        let lock = self
            .state
            .lock
            .lock
            .take()
            .ok_or(ClientError::NoSessionLock)?;
        lock.unlock_and_destroy();
        self.roundtrip()
    }

    #[must_use]
    pub fn lock_confirmed(&self) -> bool {
        self.state.lock.confirmed
    }

    #[must_use]
    pub fn lock_refused(&self) -> bool {
        self.state.lock.refused
    }

    #[must_use]
    pub fn keyboard_on_lock_surface(&self) -> bool {
        self.state.seat.keyboard_on_lock
    }
}

use wayland_client::{Connection, Dispatch, QueueHandle, delegate_noop};
use wayland_protocols::wp::pointer_constraints::zv1::client::zwp_locked_pointer_v1::{
    self, ZwpLockedPointerV1,
};
use wayland_protocols::wp::pointer_constraints::zv1::client::zwp_pointer_constraints_v1::{
    Lifetime, ZwpPointerConstraintsV1,
};
use wayland_protocols::wp::relative_pointer::zv1::client::zwp_relative_pointer_manager_v1::ZwpRelativePointerManagerV1;
use wayland_protocols::wp::relative_pointer::zv1::client::zwp_relative_pointer_v1::{
    self, ZwpRelativePointerV1,
};

use crate::client::{Client, ToplevelId};
use crate::error::ClientError;
use crate::state::ClientState;

#[derive(Debug, Default)]
pub(crate) struct PointerLockLog {
    lock: Option<ZwpLockedPointerV1>,
    relative: Option<ZwpRelativePointerV1>,
    locked: bool,
    travelled: (f64, f64),
}

delegate_noop!(ClientState: ZwpPointerConstraintsV1);
delegate_noop!(ClientState: ZwpRelativePointerManagerV1);

impl Dispatch<ZwpLockedPointerV1, ()> for ClientState {
    fn event(
        state: &mut Self,
        _lock: &ZwpLockedPointerV1,
        event: zwp_locked_pointer_v1::Event,
        (): &(),
        _connection: &Connection,
        _queue: &QueueHandle<Self>,
    ) {
        match event {
            zwp_locked_pointer_v1::Event::Locked => state.pointer_lock.locked = true,
            zwp_locked_pointer_v1::Event::Unlocked => state.pointer_lock.locked = false,
            _ => {}
        }
    }
}

impl Dispatch<ZwpRelativePointerV1, ()> for ClientState {
    fn event(
        state: &mut Self,
        _pointer: &ZwpRelativePointerV1,
        event: zwp_relative_pointer_v1::Event,
        (): &(),
        _connection: &Connection,
        _queue: &QueueHandle<Self>,
    ) {
        if let zwp_relative_pointer_v1::Event::RelativeMotion { dx, dy, .. } = event {
            let (travelled_x, travelled_y) = state.pointer_lock.travelled;
            state.pointer_lock.travelled = (travelled_x + dx, travelled_y + dy);
        }
    }
}

impl Client {
    pub fn lock_pointer(&mut self, toplevel: ToplevelId) -> Result<(), ClientError> {
        let pointer = self
            .state
            .seat
            .pointer
            .clone()
            .ok_or(ClientError::PointerOutside)?;
        let constraints: ZwpPointerConstraintsV1 =
            self.bind(1..=1, "zwp_pointer_constraints_v1")?;
        let relative: ZwpRelativePointerManagerV1 =
            self.bind(1..=1, "zwp_relative_pointer_manager_v1")?;
        let handle = self.queue.handle();
        let surface = self.surface_of(toplevel)?;
        let lock =
            constraints.lock_pointer(surface, &pointer, None, Lifetime::Persistent, &handle, ());
        let relative = relative.get_relative_pointer(&pointer, &handle, ());
        self.state.pointer_lock.lock = Some(lock);
        self.state.pointer_lock.relative = Some(relative);
        self.roundtrip()
    }

    #[must_use]
    pub fn pointer_locked(&self) -> bool {
        self.state.pointer_lock.locked
    }

    #[must_use]
    pub fn relative_travel(&self) -> (f64, f64) {
        self.state.pointer_lock.travelled
    }
}

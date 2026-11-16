use smithay::reexports::wayland_server::protocol::wl_output::WlOutput;
use smithay::wayland::session_lock::{
    LockSurface, SessionLockHandler, SessionLockManagerState, SessionLocker,
};

use crate::runtime::Compositor;

impl SessionLockHandler for Compositor {
    fn lock_state(&mut self) -> &mut SessionLockManagerState {
        &mut self.protocols.session_lock
    }

    fn lock(&mut self, confirmation: SessionLocker) {
        self.lock_for_client(confirmation);
    }

    fn unlock(&mut self) {
        self.unlock_session();
    }

    fn new_surface(&mut self, surface: LockSurface, output: WlOutput) {
        self.admit_lock_surface(surface, &output);
    }
}

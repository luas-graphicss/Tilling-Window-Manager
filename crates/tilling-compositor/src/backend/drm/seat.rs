use smithay::backend::session::Session;
use smithay::backend::session::libseat::LibSeatSession;
use smithay::reexports::calloop::LoopHandle;

use crate::error::{CompositorError, cause_chain};
use crate::runtime::Runtime;

pub(super) struct Seat {
    session: LibSeatSession,
    name: String,
}

impl Seat {
    pub(super) fn open(
        loop_handle: &LoopHandle<'static, Runtime>,
    ) -> Result<Self, CompositorError> {
        let (session, notifier) =
            LibSeatSession::new().map_err(|error| CompositorError::Seat(cause_chain(&error)))?;
        loop_handle
            .insert_source(notifier, |event, &mut (), runtime| {
                runtime.with_drm(|backend, compositor| backend.handle_session(event, compositor));
            })
            .map_err(|error| CompositorError::EventSource(error.error))?;
        let name = session.seat();
        Ok(Self { session, name })
    }

    pub(super) fn name(&self) -> &str {
        &self.name
    }

    pub(super) fn session(&mut self) -> &mut LibSeatSession {
        &mut self.session
    }

    pub(super) fn device_opener(&self) -> LibSeatSession {
        self.session.clone()
    }

    pub(super) fn is_active(&self) -> bool {
        self.session.is_active()
    }

    pub(super) fn switch_terminal(&mut self, terminal: i32) {
        if let Err(error) = self.session.change_vt(terminal) {
            tracing::warn!(error = %cause_chain(&error), terminal, "switch virtual terminal");
        }
    }
}

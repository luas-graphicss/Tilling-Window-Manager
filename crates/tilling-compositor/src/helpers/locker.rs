use std::io;
use std::time::{Duration, Instant};

use super::process;
use super::supervisor::LockerReview;
use crate::error::cause_chain;
use crate::runtime::Compositor;
use crate::security::Capability;

#[derive(Debug, thiserror::Error)]
pub(crate) enum LockRequestError {
    #[error("no locker is configured in `session.locker`")]
    NoLocker,
    #[error("start the locker `{program}`")]
    Launch {
        program: String,
        #[source]
        source: io::Error,
    },
    #[error("the locker ended without locking the session")]
    NeverLocked,
    #[error("the locker did not lock the session within {0:?}")]
    TimedOut(Duration),
    #[error("too many lock requests are waiting for the locker")]
    Crowded,
}

impl Compositor {
    pub(crate) fn request_lock(&mut self) -> Result<(), LockRequestError> {
        if self.lock.is_held() || self.helpers.locker_is_running() {
            return Ok(());
        }
        self.launch_locker()
    }

    pub(crate) fn review_locker(&mut self) {
        let engaged = self.lock.is_engaged();
        let held = self.lock.is_held();
        match self.helpers.review_locker(engaged, held, Instant::now()) {
            LockerReview::Settled => {}
            LockerReview::EndedBeforeLocking => self.report_locker_that_never_locked(),
            LockerReview::MustReturn => self.replace_locker(),
        }
        if self.lock.is_confirmed() {
            self.accept_lock_requests();
        }
    }

    fn report_locker_that_never_locked(&mut self) {
        tracing::warn!("the locker ended without locking the session");
        self.refuse_lock_requests(LockRequestError::NeverLocked);
    }

    fn replace_locker(&mut self) {
        if let Err(error) = self.launch_locker() {
            tracing::error!(
                error = %cause_chain(&error),
                "replace the locker of a locked session"
            );
        }
    }

    fn launch_locker(&mut self) -> Result<(), LockRequestError> {
        let command = self
            .config
            .session()
            .locker
            .clone()
            .ok_or(LockRequestError::NoLocker)?;
        let failed = |source| LockRequestError::Launch {
            program: command.program().to_owned(),
            source,
        };
        let privileges = [Capability::SessionLock].into_iter().collect();
        let connection = self.connect_privileged(privileges).map_err(failed)?;
        let child = process::spawn(&command, self.helpers.environment(), Some(connection))
            .map_err(failed)?;
        if let Err(error) = self.on_exit(&child, Self::review_locker) {
            tracing::debug!(%error, "watch the locker");
        }
        self.helpers.keep_locker(child);
        Ok(())
    }
}

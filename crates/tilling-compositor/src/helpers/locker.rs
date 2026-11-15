use std::io;
use std::time::Instant;

use super::process;
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
        if !self
            .helpers
            .locker_must_return(engaged, held, Instant::now())
        {
            return;
        }
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

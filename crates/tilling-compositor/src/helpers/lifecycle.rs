use std::io;
use std::process::Child;
use std::time::Instant;

use rustix::process::{Pid, PidfdFlags, pidfd_open};
use smithay::reexports::calloop::generic::Generic;
use smithay::reexports::calloop::timer::{TimeoutAction, Timer};
use smithay::reexports::calloop::{Interest, Mode, PostAction};
use tilling_config::{Helper, HelperCapability};

use super::process;
use super::supervisor::{ProcessId, STOP_GRACE};
use crate::runtime::Compositor;
use crate::security::Capability;

impl Compositor {
    pub(crate) fn sync_helpers(&mut self) {
        let wanted = self.config.session().helpers.clone();
        if self.helpers.retire_all_but(&wanted) {
            self.end_lingering_helpers_later();
        }
        for definition in self.helpers.missing(&wanted) {
            self.start_helper(&definition);
        }
    }

    pub(super) fn on_exit(
        &self,
        child: &Child,
        mut react: impl FnMut(&mut Self) + 'static,
    ) -> io::Result<()> {
        let exit = pidfd_open(Pid::from_child(child), PidfdFlags::empty())?;
        let source = Generic::new(exit, Interest::READ, Mode::Level);
        self.loop_handle
            .insert_source(source, move |_, _, runtime| {
                react(&mut runtime.compositor);
                Ok(PostAction::Remove)
            })
            .map(drop)
            .map_err(|error| io::Error::from(error.error))
    }

    fn start_helper(&mut self, definition: &Helper) {
        let child = match self.spawn_helper(definition) {
            Ok(child) => child,
            Err(error) => {
                tracing::warn!(%error, helper = %definition.name, "start a helper");
                self.helpers.give_up(definition);
                return;
            }
        };
        let id = self.helpers.next_id();
        let watched = self.on_exit(&child, move |compositor| compositor.helper_departed(id));
        if let Err(error) = watched {
            tracing::warn!(%error, helper = %definition.name, "watch a helper");
        }
        self.helpers.admit(id, definition, child);
    }

    fn spawn_helper(&mut self, definition: &Helper) -> io::Result<Child> {
        let connection = if definition.capabilities.is_empty() {
            None
        } else {
            let privileges = definition
                .capabilities
                .iter()
                .copied()
                .map(Capability::from)
                .collect();
            Some(self.connect_privileged(privileges)?)
        };
        process::spawn(&definition.command, self.helpers.environment(), connection)
    }

    fn helper_departed(&mut self, id: ProcessId) {
        let Some(definition) = self.helpers.departed(id, Instant::now()) else {
            return;
        };
        let timer = Timer::from_duration(self.helpers.restart_delay());
        let scheduled = self
            .loop_handle
            .insert_source(timer, move |_, (), runtime| {
                runtime.compositor.restart_helper(&definition);
                TimeoutAction::Drop
            });
        if let Err(error) = scheduled {
            tracing::warn!(error = %error.error, "schedule the restart of a helper");
        }
    }

    fn restart_helper(&mut self, definition: &Helper) {
        if self.helpers.awaits_restart(definition) {
            self.start_helper(definition);
        }
    }

    fn end_lingering_helpers_later(&self) {
        let timer = Timer::from_duration(STOP_GRACE);
        let scheduled = self.loop_handle.insert_source(timer, |_, (), runtime| {
            runtime.compositor.helpers.end_lingering();
            TimeoutAction::Drop
        });
        if let Err(error) = scheduled {
            tracing::warn!(error = %error.error, "schedule the end of retired helpers");
        }
    }
}

impl From<HelperCapability> for Capability {
    fn from(capability: HelperCapability) -> Self {
        match capability {
            HelperCapability::DataControl => Self::DataControl,
            HelperCapability::InputMethod => Self::InputMethod,
            HelperCapability::VirtualKeyboard => Self::VirtualKeyboard,
        }
    }
}

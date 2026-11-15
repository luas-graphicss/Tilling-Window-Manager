use std::time::Duration;

#[cfg(feature = "drm")]
use smithay::output::Mode;
use smithay::output::Output;
use smithay::reexports::calloop::timer::{TimeoutAction, Timer};
use smithay::reexports::calloop::{self, RegistrationToken};
use tilling_core::{Command, OutputId, TransitionError};

use super::settings::{OutputChange, OutputSettings};
use crate::error::{CompositorError, cause_chain};
use crate::runtime::Compositor;

#[derive(Debug, thiserror::Error)]
pub(crate) enum OutputChangeError {
    #[error("the request changes nothing")]
    Empty,
    #[error("the output is no longer connected")]
    Disconnected,
    #[error("the output does not offer that mode")]
    ModeNotOffered,
    #[error("another output change is waiting for confirmation")]
    AwaitingConfirmation,
    #[error("no output change is waiting for confirmation")]
    NothingPending,
    #[error("the output would have no usable area")]
    Area(#[source] CompositorError),
    #[error(transparent)]
    Desktop(#[from] TransitionError),
    #[error("arm the confirmation deadline")]
    Deadline(#[source] calloop::Error),
}

#[derive(Debug)]
pub(crate) struct OutputTrial {
    output: OutputId,
    previous: OutputSettings,
    deadline: RegistrationToken,
}

impl Compositor {
    pub(crate) fn try_output_change(
        &mut self,
        id: OutputId,
        change: OutputChange,
    ) -> Result<Duration, OutputChangeError> {
        if change.is_empty() {
            return Err(OutputChangeError::Empty);
        }
        if self.output_trial.is_some() {
            return Err(OutputChangeError::AwaitingConfirmation);
        }
        let (output, previous) = self
            .output_settings(id)
            .ok_or(OutputChangeError::Disconnected)?;
        let wanted = previous
            .changed_by(change, &output.modes())
            .ok_or(OutputChangeError::ModeNotOffered)?;
        self.apply_output_settings(id, wanted)?;
        match self.arm_output_deadline() {
            Ok(deadline) => {
                self.output_trial = Some(OutputTrial {
                    output: id,
                    previous,
                    deadline,
                });
                Ok(self.output_confirmation)
            }
            Err(error) => {
                self.restore_output(id, previous);
                Err(OutputChangeError::Deadline(error))
            }
        }
    }

    pub(crate) fn confirm_output_change(&mut self) -> Result<(), OutputChangeError> {
        let trial = self
            .output_trial
            .take()
            .ok_or(OutputChangeError::NothingPending)?;
        self.loop_handle.remove(trial.deadline);
        Ok(())
    }

    pub(crate) fn revert_output_change(&mut self) -> Result<(), OutputChangeError> {
        let trial = self
            .output_trial
            .take()
            .ok_or(OutputChangeError::NothingPending)?;
        self.loop_handle.remove(trial.deadline);
        self.restore_output(trial.output, trial.previous);
        Ok(())
    }

    #[cfg(feature = "drm")]
    pub(crate) fn keep_output_mode(&mut self, output: &Output, programmed: Mode) {
        let Some(id) = self.outputs.id_of(output) else {
            return;
        };
        if let Some(trial) = self.output_trial.take_if(|trial| trial.output == id) {
            self.loop_handle.remove(trial.deadline);
            self.restore_output(id, trial.previous);
        }
        let Some((_, current)) = self.output_settings(id) else {
            return;
        };
        if current.mode != programmed {
            let kept = OutputSettings {
                mode: programmed,
                ..current
            };
            self.restore_output(id, kept);
        }
    }

    pub(super) fn forget_output_trial(&mut self, id: OutputId) {
        if let Some(trial) = self.output_trial.take_if(|trial| trial.output == id) {
            self.loop_handle.remove(trial.deadline);
        }
    }

    fn output_deadline_elapsed(&mut self) {
        if let Some(trial) = self.output_trial.take() {
            tracing::info!("revert the output change that was not confirmed");
            self.restore_output(trial.output, trial.previous);
        }
    }

    fn restore_output(&mut self, id: OutputId, settings: OutputSettings) {
        if let Err(error) = self.apply_output_settings(id, settings) {
            let error = cause_chain(&error);
            tracing::warn!(error, "output settings were not restored");
        }
    }

    fn apply_output_settings(
        &mut self,
        id: OutputId,
        wanted: OutputSettings,
    ) -> Result<(), OutputChangeError> {
        let (output, current) = self
            .output_settings(id)
            .ok_or(OutputChangeError::Disconnected)?;
        let area = wanted.area().map_err(OutputChangeError::Area)?;
        wanted.apply_to(&output);
        let configured = self.execute(Command::ConfigureOutput {
            output: id,
            area,
            scale: wanted.scale,
        });
        if let Err(error) = configured {
            current.apply_to(&output);
            return Err(error.into());
        }
        self.confine_pointer();
        self.demand_frame();
        Ok(())
    }

    fn output_settings(&self, id: OutputId) -> Option<(Output, OutputSettings)> {
        let output = self.outputs.by_id(id)?.clone();
        let scale = self.desktop.output(id)?.scale();
        let settings = OutputSettings::of(&output, scale)?;
        Some((output, settings))
    }

    fn arm_output_deadline(&self) -> Result<RegistrationToken, calloop::Error> {
        let timer = Timer::from_duration(self.output_confirmation);
        self.loop_handle
            .insert_source(timer, |_, (), runtime| {
                runtime.compositor.output_deadline_elapsed();
                TimeoutAction::Drop
            })
            .map_err(|error| error.error)
    }
}

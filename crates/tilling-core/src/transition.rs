mod outputs;
mod windows;
mod workspaces;

use crate::command::Command;
use crate::desktop::DesktopState;
use crate::identity::{CapacityExhausted, OutputId, WindowId, WorkspaceId};
use crate::invariant::InvariantViolation;
use crate::layout::LayoutError;
use crate::output::Output;
use crate::window::{Window, WindowMode};
use crate::workspace::Workspace;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transition {
    pub revision: u64,
    pub effects: Vec<Effect>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    OutputAdded(OutputId),
    OutputConfigured(OutputId),
    OutputRemoved(OutputId),
    WorkspaceCreated(WorkspaceId),
    WorkspaceShown {
        output: OutputId,
        workspace: WorkspaceId,
    },
    WorkspaceActivated(WorkspaceId),
    WorkspaceArranged(WorkspaceId),
    WindowOpened(WindowId),
    WindowClosed(WindowId),
    WindowMoved {
        window: WindowId,
        workspace: WorkspaceId,
    },
    WindowMetadataChanged(WindowId),
    WindowModeChanged(WindowId),
    FocusChanged {
        previous: Option<WindowId>,
        current: Option<WindowId>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TransitionError {
    #[error("window {0} does not exist")]
    UnknownWindow(WindowId),
    #[error("workspace {0} does not exist")]
    UnknownWorkspace(WorkspaceId),
    #[error("output {0} does not exist")]
    UnknownOutput(OutputId),
    #[error("workspace name must not be empty")]
    EmptyWorkspaceName,
    #[error("workspace name `{0}` is already in use")]
    DuplicateWorkspaceName(String),
    #[error("output area must not be empty")]
    EmptyOutputArea,
    #[error("window {0} must leave fullscreen before changing mode")]
    FullscreenLocked(WindowId),
    #[error("revision counter is exhausted")]
    RevisionExhausted,
    #[error(transparent)]
    Layout(#[from] LayoutError),
    #[error(transparent)]
    Capacity(#[from] CapacityExhausted),
    #[error("transition would break an invariant: {0}")]
    Invariant(#[from] InvariantViolation),
}

impl DesktopState {
    pub fn apply(&mut self, command: Command) -> Result<Transition, TransitionError> {
        let mut next = self.clone();
        let mut effects = next.execute(command)?;
        effects.extend(next.release_unfocused_fullscreen());
        if next == *self {
            return Ok(Transition {
                revision: self.revision,
                effects: Vec::new(),
            });
        }
        next.verify()?;
        let (previous, current) = (self.focused_window(), next.focused_window());
        if previous != current {
            effects.push(Effect::FocusChanged { previous, current });
        }
        next.revision = self
            .revision
            .checked_add(1)
            .ok_or(TransitionError::RevisionExhausted)?;
        *self = next;
        Ok(Transition {
            revision: self.revision,
            effects: without_repeats(effects),
        })
    }

    fn execute(&mut self, command: Command) -> Result<Vec<Effect>, TransitionError> {
        match command {
            Command::AddOutput { name, area, scale } => self.add_output(name, area, scale),
            Command::ConfigureOutput {
                output,
                area,
                scale,
            } => self.configure_output(output, area, scale),
            Command::RemoveOutput { output } => self.remove_output(output),
            Command::CreateWorkspace { name } => self.create_workspace(name),
            Command::SwitchWorkspace { workspace } => self.activate_workspace(workspace),
            Command::OpenWindow {
                metadata,
                workspace,
                transient_for,
                mode,
            } => self.open_window(metadata, workspace, transient_for, mode),
            Command::CloseWindow { window } => self.close_window(window),
            Command::UpdateWindowMetadata { window, metadata } => {
                self.update_window_metadata(window, metadata)
            }
            Command::FocusWindow { window } => self.focus_window(window),
            Command::FocusDirection { direction } => self.focus_direction(direction),
            Command::MoveWindow { window, direction } => self.move_window(window, direction),
            Command::MoveWindowToWorkspace { window, workspace } => {
                self.move_window_to_workspace(window, workspace)
            }
            Command::ChangeWindowMode { window, change } => self.change_window_mode(window, change),
            Command::ToggleSplit { window } => self.toggle_split(window),
            Command::ResizeSplit { window, delta } => self.resize_split(window, delta),
        }
    }

    fn release_unfocused_fullscreen(&mut self) -> Vec<Effect> {
        let unfocused: Vec<(WorkspaceId, WindowId)> = self
            .workspaces
            .iter()
            .flat_map(|(workspace_id, workspace)| {
                let focused = workspace.focus.current();
                workspace
                    .windows
                    .iter()
                    .filter(move |window| Some(**window) != focused)
                    .map(move |window| (workspace_id, *window))
            })
            .collect();
        let mut effects = Vec::new();
        for (workspace_id, window_id) in unfocused {
            let Some(window) = self.windows.get_mut(window_id) else {
                continue;
            };
            let WindowMode::Fullscreen(restore) = window.mode else {
                continue;
            };
            window.mode = restore.into();
            effects.push(Effect::WindowModeChanged(window_id));
            effects.push(Effect::WorkspaceArranged(workspace_id));
        }
        effects
    }

    fn require_window(&self, window: WindowId) -> Result<&Window, TransitionError> {
        self.windows
            .get(window)
            .ok_or(TransitionError::UnknownWindow(window))
    }

    fn require_window_mut(&mut self, window: WindowId) -> Result<&mut Window, TransitionError> {
        self.windows
            .get_mut(window)
            .ok_or(TransitionError::UnknownWindow(window))
    }

    fn require_workspace(&self, workspace: WorkspaceId) -> Result<&Workspace, TransitionError> {
        self.workspaces
            .get(workspace)
            .ok_or(TransitionError::UnknownWorkspace(workspace))
    }

    fn require_workspace_mut(
        &mut self,
        workspace: WorkspaceId,
    ) -> Result<&mut Workspace, TransitionError> {
        self.workspaces
            .get_mut(workspace)
            .ok_or(TransitionError::UnknownWorkspace(workspace))
    }

    fn require_output_mut(&mut self, output: OutputId) -> Result<&mut Output, TransitionError> {
        self.outputs
            .get_mut(output)
            .ok_or(TransitionError::UnknownOutput(output))
    }
}

fn without_repeats(effects: Vec<Effect>) -> Vec<Effect> {
    let mut unique = Vec::with_capacity(effects.len());
    for effect in effects {
        if !unique.contains(&effect) {
            unique.push(effect);
        }
    }
    unique
}

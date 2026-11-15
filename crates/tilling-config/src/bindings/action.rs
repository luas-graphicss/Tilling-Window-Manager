use std::collections::BTreeSet;

use tilling_core::layout::{Direction, LayoutKind};

use crate::applications::{CommandLine, CommandLineError};
use crate::limits::MAX_RESIZE_STEP;
use crate::schema::{RawAction, RawDirection};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Spawn(CommandLine),
    OpenTerminal,
    CloseWindow,
    Focus(Direction),
    Move(Direction),
    SwitchWorkspace(String),
    MoveToWorkspace(String),
    FocusOutput(Direction),
    MoveToOutput(Direction),
    ToggleFullscreen,
    ToggleFloating,
    MoveToScratchpad,
    ToggleScratchpad,
    ToggleSplit,
    Resize(i16),
    SetLayout(LayoutKind),
    CycleLayout,
    EnterMode(String),
    ReloadConfig,
    Lock,
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ActionError {
    #[error(transparent)]
    Command(#[from] CommandLineError),
    #[error("workspace `{0}` is not listed in `workspaces`")]
    UnknownWorkspace(String),
    #[error("mode `{0}` has no bindings")]
    UnknownMode(String),
    #[error("resize step must be a non-zero value within {MAX_RESIZE_STEP} basis points")]
    ResizeOutOfRange,
}

pub(crate) struct References<'a> {
    pub(crate) workspaces: &'a [String],
    pub(crate) modes: &'a BTreeSet<String>,
}

impl References<'_> {
    fn workspace(&self, name: String) -> Result<String, ActionError> {
        if self.workspaces.contains(&name) {
            Ok(name)
        } else {
            Err(ActionError::UnknownWorkspace(name))
        }
    }
}

pub(crate) fn validate(raw: RawAction, references: &References<'_>) -> Result<Action, ActionError> {
    match raw {
        RawAction::Spawn(parts) => Ok(Action::Spawn(CommandLine::from_parts(parts)?)),
        RawAction::OpenTerminal => Ok(Action::OpenTerminal),
        RawAction::CloseWindow => Ok(Action::CloseWindow),
        RawAction::Focus(direction) => Ok(Action::Focus(direction.into())),
        RawAction::Move(direction) => Ok(Action::Move(direction.into())),
        RawAction::SwitchWorkspace(name) => references.workspace(name).map(Action::SwitchWorkspace),
        RawAction::MoveToWorkspace(name) => references.workspace(name).map(Action::MoveToWorkspace),
        RawAction::FocusOutput(direction) => Ok(Action::FocusOutput(direction.into())),
        RawAction::MoveToOutput(direction) => Ok(Action::MoveToOutput(direction.into())),
        RawAction::ToggleFullscreen => Ok(Action::ToggleFullscreen),
        RawAction::ToggleFloating => Ok(Action::ToggleFloating),
        RawAction::MoveToScratchpad => Ok(Action::MoveToScratchpad),
        RawAction::ToggleScratchpad => Ok(Action::ToggleScratchpad),
        RawAction::ToggleSplit => Ok(Action::ToggleSplit),
        RawAction::Resize(step) if step == 0 || step.unsigned_abs() > MAX_RESIZE_STEP => {
            Err(ActionError::ResizeOutOfRange)
        }
        RawAction::Resize(step) => Ok(Action::Resize(step)),
        RawAction::SetLayout(layout) => Ok(Action::SetLayout(layout.into())),
        RawAction::CycleLayout => Ok(Action::CycleLayout),
        RawAction::EnterMode(mode) if references.modes.contains(&mode) => {
            Ok(Action::EnterMode(mode))
        }
        RawAction::EnterMode(mode) => Err(ActionError::UnknownMode(mode)),
        RawAction::ReloadConfig => Ok(Action::ReloadConfig),
        RawAction::Lock => Ok(Action::Lock),
        RawAction::Quit => Ok(Action::Quit),
    }
}

impl From<RawDirection> for Direction {
    fn from(raw: RawDirection) -> Self {
        match raw {
            RawDirection::Left => Self::Left,
            RawDirection::Right => Self::Right,
            RawDirection::Up => Self::Up,
            RawDirection::Down => Self::Down,
        }
    }
}

use super::{Effect, TransitionError};
use crate::command::ModeChange;
use crate::desktop::DesktopState;
use crate::identity::{WindowId, WorkspaceId};
use crate::layout::Direction;
use crate::window::{RestoreMode, Window, WindowMetadata, WindowMode};

impl DesktopState {
    pub(super) fn open_window(
        &mut self,
        metadata: WindowMetadata,
        workspace: Option<WorkspaceId>,
        transient_for: Option<WindowId>,
        mode: WindowMode,
    ) -> Result<Vec<Effect>, TransitionError> {
        let workspace = workspace.unwrap_or(self.active_workspace);
        if let Some(parent) = transient_for {
            self.require_window(parent)?;
        }
        self.require_workspace(workspace)?;
        let window = self.windows.insert(Window {
            metadata,
            workspace,
            mode,
            transient_for,
        })?;
        self.require_workspace_mut(workspace)?
            .attach(window, mode)?;
        Ok(vec![
            Effect::WindowOpened(window),
            Effect::WorkspaceArranged(workspace),
        ])
    }

    pub(super) fn close_window(
        &mut self,
        window: WindowId,
    ) -> Result<Vec<Effect>, TransitionError> {
        let closed = self
            .windows
            .remove(window)
            .ok_or(TransitionError::UnknownWindow(window))?;
        self.require_workspace_mut(closed.workspace)?.detach(window);
        for dependent in self.windows.values_mut() {
            if dependent.transient_for == Some(window) {
                dependent.transient_for = None;
            }
        }
        Ok(vec![
            Effect::WindowClosed(window),
            Effect::WorkspaceArranged(closed.workspace),
        ])
    }

    pub(super) fn update_window_metadata(
        &mut self,
        window: WindowId,
        metadata: WindowMetadata,
    ) -> Result<Vec<Effect>, TransitionError> {
        self.require_window_mut(window)?.metadata = metadata;
        Ok(vec![Effect::WindowMetadataChanged(window)])
    }

    pub(super) fn focus_window(
        &mut self,
        window: WindowId,
    ) -> Result<Vec<Effect>, TransitionError> {
        let workspace = self.require_window(window)?.workspace;
        self.require_workspace_mut(workspace)?.focus.promote(window);
        self.activate_workspace(workspace)
    }

    pub(super) fn focus_direction(
        &mut self,
        direction: Direction,
    ) -> Result<Vec<Effect>, TransitionError> {
        let Some(focused) = self.focused_window() else {
            return Ok(Vec::new());
        };
        let workspace = self.require_workspace_mut(self.active_workspace)?;
        if let Some(neighbor) = workspace.layout.neighbor(focused, direction) {
            workspace.focus.promote(neighbor);
        }
        Ok(Vec::new())
    }

    pub(super) fn move_window(
        &mut self,
        window: WindowId,
        direction: Direction,
    ) -> Result<Vec<Effect>, TransitionError> {
        let workspace_id = self.require_window(window)?.workspace;
        let workspace = self.require_workspace_mut(workspace_id)?;
        let Some(neighbor) = workspace.layout.neighbor(window, direction) else {
            return Ok(Vec::new());
        };
        workspace.layout.swap(window, neighbor);
        Ok(vec![Effect::WorkspaceArranged(workspace_id)])
    }

    pub(super) fn move_window_to_workspace(
        &mut self,
        window: WindowId,
        target: WorkspaceId,
    ) -> Result<Vec<Effect>, TransitionError> {
        let Window {
            workspace: source,
            mode,
            ..
        } = *self.require_window(window)?;
        self.require_workspace(target)?;
        if source == target {
            return Ok(Vec::new());
        }
        self.require_workspace_mut(source)?.detach(window);
        self.require_workspace_mut(target)?.attach(window, mode)?;
        self.require_window_mut(window)?.workspace = target;
        Ok(vec![
            Effect::WindowMoved {
                window,
                workspace: target,
            },
            Effect::WorkspaceArranged(source),
            Effect::WorkspaceArranged(target),
        ])
    }

    pub(super) fn change_window_mode(
        &mut self,
        window: WindowId,
        change: ModeChange,
    ) -> Result<Vec<Effect>, TransitionError> {
        let Window {
            workspace: workspace_id,
            mode: previous,
            ..
        } = *self.require_window(window)?;
        let next = match (previous, change) {
            (WindowMode::Fullscreen(restore), ModeChange::LeaveFullscreen) => restore.into(),
            (WindowMode::Fullscreen(_), ModeChange::EnterFullscreen) => return Ok(Vec::new()),
            (WindowMode::Fullscreen(_), ModeChange::Tile | ModeChange::Float(_)) => {
                return Err(TransitionError::FullscreenLocked(window));
            }
            (WindowMode::Tiled | WindowMode::Floating(_), ModeChange::LeaveFullscreen) => {
                return Ok(Vec::new());
            }
            (WindowMode::Tiled, ModeChange::EnterFullscreen) => {
                WindowMode::Fullscreen(RestoreMode::Tiled)
            }
            (WindowMode::Floating(area), ModeChange::EnterFullscreen) => {
                WindowMode::Fullscreen(RestoreMode::Floating(area))
            }
            (WindowMode::Tiled | WindowMode::Floating(_), ModeChange::Tile) => WindowMode::Tiled,
            (WindowMode::Tiled | WindowMode::Floating(_), ModeChange::Float(area)) => {
                WindowMode::Floating(area)
            }
        };
        let workspace = self.require_workspace_mut(workspace_id)?;
        match (previous.occupies_tile(), next.occupies_tile()) {
            (true, false) => {
                workspace.layout.remove(window);
            }
            (false, true) => workspace.layout.insert(workspace.focus.current(), window)?,
            (true, true) | (false, false) => {}
        }
        if matches!(next, WindowMode::Fullscreen(_)) {
            workspace.focus.promote(window);
        }
        self.require_window_mut(window)?.mode = next;
        Ok(vec![
            Effect::WindowModeChanged(window),
            Effect::WorkspaceArranged(workspace_id),
        ])
    }

    pub(super) fn toggle_split(
        &mut self,
        window: WindowId,
    ) -> Result<Vec<Effect>, TransitionError> {
        let workspace_id = self.require_window(window)?.workspace;
        let workspace = self.require_workspace_mut(workspace_id)?;
        if !workspace.layout.toggle_orientation(window) {
            return Ok(Vec::new());
        }
        Ok(vec![Effect::WorkspaceArranged(workspace_id)])
    }

    pub(super) fn resize_split(
        &mut self,
        window: WindowId,
        delta: i16,
    ) -> Result<Vec<Effect>, TransitionError> {
        let workspace_id = self.require_window(window)?.workspace;
        let workspace = self.require_workspace_mut(workspace_id)?;
        if !workspace.layout.resize(window, delta) {
            return Ok(Vec::new());
        }
        Ok(vec![Effect::WorkspaceArranged(workspace_id)])
    }
}

use super::{Effect, TransitionError};
use crate::desktop::DesktopState;
use crate::geometry::LogicalRect;
use crate::identity::WindowId;
use crate::window::WindowMode;

impl DesktopState {
    pub(super) fn stash_window(
        &mut self,
        window: WindowId,
        area: LogicalRect,
    ) -> Result<Vec<Effect>, TransitionError> {
        let placed = self.require_placed(window)?;
        let workspace = placed.workspace;
        let floating = placed.mode.floating_area().unwrap_or(area);
        self.require_workspace_mut(workspace)?.detach(window);
        self.require_window_mut(window)?.mode = WindowMode::Floating(floating);
        self.scratchpad.stash(window);
        Ok(vec![
            Effect::WindowModeChanged(window),
            Effect::WorkspaceArranged(workspace),
        ])
    }

    pub(super) fn toggle_scratchpad(&mut self) -> Result<Vec<Effect>, TransitionError> {
        let shown = self
            .focused_window()
            .filter(|window| self.scratchpad.is_summoned(*window))
            .and_then(|window| Some((window, self.windows.get(window)?.mode.floating_area()?)));
        match shown {
            Some((window, area)) => self.stash_window(window, area),
            None => self.summon_oldest_stashed(),
        }
    }

    fn summon_oldest_stashed(&mut self) -> Result<Vec<Effect>, TransitionError> {
        let Some(window) = self.scratchpad.summon_oldest() else {
            return Ok(Vec::new());
        };
        let workspace = self.active_workspace;
        let mode = self.require_window(window)?.mode;
        self.require_workspace_mut(workspace)?
            .attach(window, mode)?;
        self.require_window_mut(window)?.workspace = workspace;
        self.fit_floating_window(window)?;
        Ok(vec![
            Effect::WindowMoved { window, workspace },
            Effect::WindowModeChanged(window),
            Effect::WorkspaceArranged(workspace),
        ])
    }
}

use serde::Serialize;

use crate::identity::{Arena, OutputId, WindowId, WorkspaceId};
use crate::layout::{Gaps, LayoutKind, Placement};
use crate::output::Output;
use crate::window::{Window, WindowMode};
use crate::workspace::Workspace;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DesktopState {
    pub(crate) windows: Arena<WindowId, Window>,
    pub(crate) workspaces: Arena<WorkspaceId, Workspace>,
    pub(crate) outputs: Arena<OutputId, Output>,
    pub(crate) active_workspace: WorkspaceId,
    pub(crate) default_layout: LayoutKind,
    pub(crate) revision: u64,
}

impl DesktopState {
    #[must_use]
    pub fn new(initial_workspace: impl Into<String>) -> Self {
        let default_layout = LayoutKind::default();
        let initial = Workspace::new(initial_workspace.into(), None, default_layout);
        let (workspaces, active_workspace) = Arena::with_initial(initial);
        Self {
            windows: Arena::new(),
            workspaces,
            outputs: Arena::new(),
            active_workspace,
            default_layout,
            revision: 0,
        }
    }

    #[must_use]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    #[must_use]
    pub fn active_workspace(&self) -> WorkspaceId {
        self.active_workspace
    }

    #[must_use]
    pub fn window(&self, window: WindowId) -> Option<&Window> {
        self.windows.get(window)
    }

    #[must_use]
    pub fn workspace(&self, workspace: WorkspaceId) -> Option<&Workspace> {
        self.workspaces.get(workspace)
    }

    #[must_use]
    pub fn output(&self, output: OutputId) -> Option<&Output> {
        self.outputs.get(output)
    }

    pub fn windows(&self) -> impl Iterator<Item = (WindowId, &Window)> {
        self.windows.iter()
    }

    pub fn workspaces(&self) -> impl Iterator<Item = (WorkspaceId, &Workspace)> {
        self.workspaces.iter()
    }

    pub fn outputs(&self) -> impl Iterator<Item = (OutputId, &Output)> {
        self.outputs.iter()
    }

    #[must_use]
    pub fn focused_window(&self) -> Option<WindowId> {
        let workspace = self.workspaces.get(self.active_workspace)?;
        let output = self.outputs.get(workspace.output?)?;
        if output.workspace != self.active_workspace {
            return None;
        }
        workspace.focus.current()
    }

    #[must_use]
    pub fn placements(&self, workspace: WorkspaceId, gaps: Gaps) -> Option<Vec<Placement>> {
        let workspace = self.workspaces.get(workspace)?;
        let Some(output) = workspace.output.and_then(|owner| self.outputs.get(owner)) else {
            return Some(Vec::new());
        };
        let mode_of = |window: WindowId| self.windows.get(window).map(|window| window.mode);
        let tiles = workspace
            .layout
            .arrange(
                output.area.shrink(gaps.outer),
                gaps.inner,
                workspace.front_tile(),
            )
            .into_iter()
            .filter(|placement| mode_of(placement.window) == Some(WindowMode::Tiled));
        let floating = workspace.windows.iter().filter_map(|window| {
            let WindowMode::Floating(area) = mode_of(*window)? else {
                return None;
            };
            Some(Placement {
                window: *window,
                area,
            })
        });
        let fullscreen = workspace
            .windows
            .iter()
            .filter(|window| matches!(mode_of(**window), Some(WindowMode::Fullscreen(_))))
            .map(|window| Placement {
                window: *window,
                area: output.area,
            });
        Some(tiles.chain(floating).chain(fullscreen).collect())
    }
}

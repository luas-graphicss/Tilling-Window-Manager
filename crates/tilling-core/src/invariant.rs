use crate::desktop::DesktopState;
use crate::identity::{OutputId, WindowId, WorkspaceId};
use crate::window::WindowMode;
use crate::workspace::Workspace;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvariantViolation {
    #[error("active workspace {0} does not exist")]
    MissingActiveWorkspace(WorkspaceId),
    #[error("active workspace {0} is not shown on any output")]
    HiddenActiveWorkspace(WorkspaceId),
    #[error("workspaces list {listed} windows but {existing} exist")]
    WindowCountMismatch { listed: usize, existing: usize },
    #[error("window {0} is transient for a window that does not exist")]
    DanglingTransientParent(WindowId),
    #[error("workspace {0} lists a window it does not own exactly once")]
    ForeignWindow(WorkspaceId),
    #[error("workspace {0} focus order does not match its windows")]
    FocusOrderMismatch(WorkspaceId),
    #[error("workspace {0} layout does not match its tiled windows")]
    LayoutMismatch(WorkspaceId),
    #[error("workspace {0} keeps a fullscreen window that is not focused")]
    UnfocusedFullscreen(WorkspaceId),
    #[error("workspace {0} belongs to an output that does not exist")]
    DanglingOwner(WorkspaceId),
    #[error("workspace name `{0}` is not unique")]
    DuplicateWorkspaceName(String),
    #[error("output {0} shows a workspace it does not own")]
    OutputWorkspaceMismatch(OutputId),
}

impl DesktopState {
    pub fn verify(&self) -> Result<(), InvariantViolation> {
        self.verify_active_workspace()?;
        self.verify_windows()?;
        for (id, workspace) in self.workspaces.iter() {
            self.verify_workspace(id, workspace)?;
        }
        self.verify_workspace_names()?;
        self.verify_outputs()
    }

    fn verify_active_workspace(&self) -> Result<(), InvariantViolation> {
        let active = self.active_workspace;
        let workspace = self
            .workspaces
            .get(active)
            .ok_or(InvariantViolation::MissingActiveWorkspace(active))?;
        if self.outputs.len() == 0 {
            return Ok(());
        }
        let shown = workspace
            .output
            .and_then(|owner| self.outputs.get(owner))
            .is_some_and(|owner| owner.workspace == active);
        if shown {
            Ok(())
        } else {
            Err(InvariantViolation::HiddenActiveWorkspace(active))
        }
    }

    fn verify_windows(&self) -> Result<(), InvariantViolation> {
        let listed = self
            .workspaces
            .iter()
            .map(|(_, workspace)| workspace.windows.len())
            .sum();
        let existing = self.windows.len();
        if listed != existing {
            return Err(InvariantViolation::WindowCountMismatch { listed, existing });
        }
        let dangling = self.windows.iter().find(|(id, window)| {
            window
                .transient_for
                .is_some_and(|parent| parent == *id || !self.windows.contains(parent))
        });
        match dangling {
            Some((id, _)) => Err(InvariantViolation::DanglingTransientParent(id)),
            None => Ok(()),
        }
    }

    fn verify_workspace(
        &self,
        id: WorkspaceId,
        workspace: &Workspace,
    ) -> Result<(), InvariantViolation> {
        let members = sorted(workspace.windows.clone());
        let unique = members
            .iter()
            .zip(members.iter().skip(1))
            .all(|(a, b)| a != b);
        let mode_of = |window: WindowId| {
            self.windows
                .get(window)
                .filter(|entry| entry.workspace == id)
                .map(|entry| entry.mode)
        };
        if !unique || members.iter().any(|window| mode_of(*window).is_none()) {
            return Err(InvariantViolation::ForeignWindow(id));
        }
        if sorted(workspace.focus.windows().to_vec()) != members {
            return Err(InvariantViolation::FocusOrderMismatch(id));
        }
        let tiled: Vec<WindowId> = members
            .iter()
            .copied()
            .filter(|window| mode_of(*window).is_some_and(WindowMode::occupies_tile))
            .collect();
        if sorted(workspace.layout.windows()) != tiled {
            return Err(InvariantViolation::LayoutMismatch(id));
        }
        let focused = workspace.focus.current();
        let unfocused_fullscreen = members.iter().any(|window| {
            Some(*window) != focused && matches!(mode_of(*window), Some(WindowMode::Fullscreen(_)))
        });
        if unfocused_fullscreen {
            return Err(InvariantViolation::UnfocusedFullscreen(id));
        }
        if workspace
            .output
            .is_some_and(|owner| !self.outputs.contains(owner))
        {
            return Err(InvariantViolation::DanglingOwner(id));
        }
        Ok(())
    }

    fn verify_workspace_names(&self) -> Result<(), InvariantViolation> {
        let mut names: Vec<&str> = self
            .workspaces
            .iter()
            .map(|(_, workspace)| workspace.name.as_str())
            .collect();
        names.sort_unstable();
        let repeated = names.iter().zip(names.iter().skip(1)).find(|(a, b)| a == b);
        match repeated {
            Some((name, _)) => Err(InvariantViolation::DuplicateWorkspaceName(
                (*name).to_owned(),
            )),
            None => Ok(()),
        }
    }

    fn verify_outputs(&self) -> Result<(), InvariantViolation> {
        let mismatched = self.outputs.iter().find(|(id, output)| {
            self.workspaces
                .get(output.workspace)
                .is_none_or(|workspace| workspace.output != Some(*id))
        });
        match mismatched {
            Some((id, _)) => Err(InvariantViolation::OutputWorkspaceMismatch(id)),
            None => Ok(()),
        }
    }
}

fn sorted(mut windows: Vec<WindowId>) -> Vec<WindowId> {
    windows.sort_unstable();
    windows
}

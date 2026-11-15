use super::{Effect, TransitionError};
use crate::desktop::DesktopState;
use crate::identity::WorkspaceId;
use crate::workspace::Workspace;

impl DesktopState {
    pub(super) fn create_workspace(
        &mut self,
        name: String,
    ) -> Result<Vec<Effect>, TransitionError> {
        if name.is_empty() {
            return Err(TransitionError::EmptyWorkspaceName);
        }
        if self.workspace_name_in_use(&name) {
            return Err(TransitionError::DuplicateWorkspaceName(name));
        }
        let owner = self.require_workspace(self.active_workspace)?.output;
        let created = self.workspaces.insert(Workspace::new(name, owner))?;
        Ok(vec![Effect::WorkspaceCreated(created)])
    }

    pub(super) fn activate_workspace(
        &mut self,
        workspace: WorkspaceId,
    ) -> Result<Vec<Effect>, TransitionError> {
        let active_owner = self.require_workspace(self.active_workspace)?.output;
        let target = self.require_workspace_mut(workspace)?;
        let owner = target.output.or(active_owner);
        target.output = owner;
        let mut effects = Vec::new();
        if let Some(output) = owner {
            let shown = &mut self.require_output_mut(output)?.workspace;
            if *shown != workspace {
                *shown = workspace;
                effects.push(Effect::WorkspaceShown { output, workspace });
            }
        }
        if self.active_workspace != workspace {
            self.active_workspace = workspace;
            effects.push(Effect::WorkspaceActivated(workspace));
        }
        Ok(effects)
    }

    pub(super) fn workspace_name_in_use(&self, name: &str) -> bool {
        self.workspaces
            .iter()
            .any(|(_, workspace)| workspace.name == name)
    }
}

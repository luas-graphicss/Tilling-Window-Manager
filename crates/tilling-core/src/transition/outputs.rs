use super::{Effect, TransitionError};
use crate::desktop::DesktopState;
use crate::geometry::{LogicalRect, Scale};
use crate::identity::{OutputId, WorkspaceId};
use crate::output::Output;
use crate::workspace::{FormerOutput, Workspace};

impl DesktopState {
    pub(super) fn add_output(
        &mut self,
        name: String,
        area: LogicalRect,
        scale: Scale,
    ) -> Result<Vec<Effect>, TransitionError> {
        if area.is_empty() {
            return Err(TransitionError::EmptyOutputArea);
        }
        let mut effects = Vec::new();
        let returning = self.workspaces_returning_to(&name);
        let workspace = if let Some(existing) = self.workspace_for_new_output(&returning) {
            existing
        } else {
            let name = self.unused_workspace_name();
            let workspace = Workspace::new(name, None, self.default_layout);
            let created = self.workspaces.insert(workspace)?;
            effects.push(Effect::WorkspaceCreated(created));
            created
        };
        let output = self.outputs.insert(Output {
            name,
            area,
            scale,
            workspace,
        })?;
        for owned in returning.into_iter().chain([workspace]) {
            let owned = self.require_workspace_mut(owned)?;
            owned.output = Some(output);
            owned.former_output = None;
        }
        effects.push(Effect::OutputAdded(output));
        effects.push(Effect::WorkspaceShown { output, workspace });
        Ok(effects)
    }

    pub(super) fn configure_output(
        &mut self,
        output: OutputId,
        area: LogicalRect,
        scale: Scale,
    ) -> Result<Vec<Effect>, TransitionError> {
        if area.is_empty() {
            return Err(TransitionError::EmptyOutputArea);
        }
        let configured = self.require_output_mut(output)?;
        configured.area = area;
        configured.scale = scale;
        Ok(vec![Effect::OutputConfigured(output)])
    }

    pub(super) fn remove_output(
        &mut self,
        output: OutputId,
    ) -> Result<Vec<Effect>, TransitionError> {
        let removed = self
            .outputs
            .remove(output)
            .ok_or(TransitionError::UnknownOutput(output))?;
        let heir = self.outputs.iter().next().map(|(heir, _)| heir);
        let orphaned: Vec<WorkspaceId> = self
            .workspaces
            .iter()
            .filter(|(_, workspace)| workspace.output == Some(output))
            .map(|(id, _)| id)
            .collect();
        for id in orphaned {
            let workspace = self.require_workspace_mut(id)?;
            workspace.output = heir;
            workspace.former_output = Some(FormerOutput {
                name: removed.name.clone(),
                showing: id == removed.workspace,
            });
        }
        let mut effects = vec![Effect::OutputRemoved(output)];
        let active = self.active_workspace;
        if let Some(heir) = heir
            && removed.workspace == active
        {
            self.require_output_mut(heir)?.workspace = active;
            effects.push(Effect::WorkspaceShown {
                output: heir,
                workspace: active,
            });
        }
        Ok(effects)
    }

    fn workspaces_returning_to(&self, name: &str) -> Vec<WorkspaceId> {
        self.workspaces
            .iter()
            .filter(|(id, workspace)| {
                let left_this_output = workspace
                    .former_output
                    .as_ref()
                    .is_some_and(|former| former.name == name);
                left_this_output && !self.is_shown(*id)
            })
            .map(|(id, _)| id)
            .collect()
    }

    fn workspace_for_new_output(&self, returning: &[WorkspaceId]) -> Option<WorkspaceId> {
        let active = self.workspaces.get(self.active_workspace)?;
        if active.output.is_none() {
            return Some(self.active_workspace);
        }
        let was_showing = |id: &&WorkspaceId| {
            self.workspaces
                .get(**id)
                .and_then(|workspace| workspace.former_output.as_ref())
                .is_some_and(|former| former.showing)
        };
        returning
            .iter()
            .find(was_showing)
            .or_else(|| returning.first())
            .copied()
            .or_else(|| self.parked_workspace())
    }

    fn parked_workspace(&self) -> Option<WorkspaceId> {
        self.workspaces
            .iter()
            .find(|(_, workspace)| workspace.output.is_none())
            .map(|(id, _)| id)
    }

    fn is_shown(&self, workspace: WorkspaceId) -> bool {
        self.outputs
            .iter()
            .any(|(_, output)| output.workspace == workspace)
    }

    fn unused_workspace_name(&self) -> String {
        let mut number = self.workspaces.len() + 1;
        loop {
            let name = number.to_string();
            if !self.workspace_name_in_use(&name) {
                return name;
            }
            number += 1;
        }
    }
}

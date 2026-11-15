use super::{Effect, TransitionError};
use crate::desktop::DesktopState;
use crate::geometry::{LogicalRect, Scale};
use crate::identity::{OutputId, WorkspaceId};
use crate::output::Output;
use crate::workspace::Workspace;

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
        let workspace = if let Some(parked) = self.parked_workspace() {
            parked
        } else {
            let name = self.unused_workspace_name();
            let created = self.workspaces.insert(Workspace::new(name, None))?;
            effects.push(Effect::WorkspaceCreated(created));
            created
        };
        let output = self.outputs.insert(Output {
            name,
            area,
            scale,
            workspace,
        })?;
        self.require_workspace_mut(workspace)?.output = Some(output);
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
        for workspace in self.workspaces.values_mut() {
            if workspace.output == Some(output) {
                workspace.output = heir;
            }
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

    fn parked_workspace(&self) -> Option<WorkspaceId> {
        let is_parked = |workspace: &Workspace| workspace.output.is_none();
        if self
            .workspaces
            .get(self.active_workspace)
            .is_some_and(is_parked)
        {
            return Some(self.active_workspace);
        }
        self.workspaces
            .iter()
            .find(|(_, workspace)| is_parked(workspace))
            .map(|(id, _)| id)
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

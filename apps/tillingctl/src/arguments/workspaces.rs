use clap::{Subcommand, ValueEnum};
use tilling_ipc::{Call, ResourceId, WorkspaceInfo};

use super::parse_resource_id;

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum LayoutAction {
    #[command(about = "Choose the layout of a workspace")]
    Set {
        #[arg(value_enum)]
        layout: Layout,
        #[arg(
            long,
            value_name = "WORKSPACE",
            value_parser = parse_workspace,
            help = "Name or identifier of the workspace, the active one when omitted"
        )]
        workspace: Option<WorkspaceReference>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum WorkspaceAction {
    #[command(about = "Switch to a workspace by name or identifier")]
    Switch {
        #[arg(value_name = "WORKSPACE", value_parser = parse_workspace)]
        workspace: WorkspaceReference,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Layout {
    Bsp,
    MasterStack,
    Monocle,
}

impl From<Layout> for tilling_ipc::Layout {
    fn from(layout: Layout) -> Self {
        match layout {
            Layout::Bsp => Self::Bsp,
            Layout::MasterStack => Self::MasterStack,
            Layout::Monocle => Self::Monocle,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceReference {
    Active,
    Identifier(ResourceId),
    Named(String),
}

impl WorkspaceReference {
    pub fn find(&self, workspaces: &[WorkspaceInfo]) -> Option<ResourceId> {
        let wanted = |workspace: &&WorkspaceInfo| match self {
            Self::Active => workspace.active,
            Self::Identifier(identifier) => workspace.id == *identifier,
            Self::Named(name) => workspace.name == *name,
        };
        workspaces.iter().find(wanted).map(|workspace| workspace.id)
    }

    pub fn missing(&self) -> String {
        match self {
            Self::Active => "the compositor reported no active workspace".to_owned(),
            Self::Identifier(identifier) => format!(
                "workspace {}:{} does not exist",
                identifier.index, identifier.generation
            ),
            Self::Named(name) => format!("workspace `{name}` does not exist"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceCall {
    Switch,
    SetLayout(tilling_ipc::Layout),
}

impl WorkspaceCall {
    pub fn on(self, workspace: ResourceId) -> Call {
        match self {
            Self::Switch => Call::SwitchWorkspace { workspace },
            Self::SetLayout(layout) => Call::SetLayout { workspace, layout },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("expected a workspace name or INDEX:GENERATION")]
pub struct EmptyWorkspaceReference;

fn parse_workspace(text: &str) -> Result<WorkspaceReference, EmptyWorkspaceReference> {
    if text.is_empty() {
        return Err(EmptyWorkspaceReference);
    }
    Ok(parse_resource_id(text).map_or_else(
        |_| WorkspaceReference::Named(text.to_owned()),
        WorkspaceReference::Identifier,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECOND: ResourceId = ResourceId {
        index: 1,
        generation: 0,
    };

    fn workspace(index: u32, name: &str, active: bool) -> WorkspaceInfo {
        WorkspaceInfo {
            id: ResourceId {
                index,
                generation: 0,
            },
            name: name.to_owned(),
            output: None,
            focused_window: None,
            windows: Vec::new(),
            layout: tilling_ipc::Layout::Bsp,
            active,
        }
    }

    #[test]
    fn references_are_names_unless_they_look_like_identifiers() {
        assert_eq!(
            parse_workspace("2"),
            Ok(WorkspaceReference::Named("2".to_owned()))
        );
        assert_eq!(
            parse_workspace("1:0"),
            Ok(WorkspaceReference::Identifier(SECOND))
        );
        assert_eq!(parse_workspace(""), Err(EmptyWorkspaceReference));
    }

    #[test]
    fn references_find_the_workspace_they_describe() {
        let listed = [workspace(0, "web", false), workspace(1, "code", true)];

        assert_eq!(WorkspaceReference::Active.find(&listed), Some(SECOND));
        assert_eq!(
            WorkspaceReference::Named("code".to_owned()).find(&listed),
            Some(SECOND)
        );
        assert_eq!(
            WorkspaceReference::Identifier(SECOND).find(&listed),
            Some(SECOND)
        );
        assert_eq!(
            WorkspaceReference::Named("mail".to_owned()).find(&listed),
            None
        );
        assert_eq!(WorkspaceReference::Active.find(&listed[..1]), None);
    }

    #[test]
    fn workspace_calls_target_the_resolved_workspace() {
        assert_eq!(
            WorkspaceCall::Switch.on(SECOND),
            Call::SwitchWorkspace { workspace: SECOND }
        );
        assert_eq!(
            WorkspaceCall::SetLayout(tilling_ipc::Layout::Monocle).on(SECOND),
            Call::SetLayout {
                workspace: SECOND,
                layout: tilling_ipc::Layout::Monocle,
            }
        );
    }
}

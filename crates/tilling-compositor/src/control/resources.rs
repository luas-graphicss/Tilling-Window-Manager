use tilling_core::{DesktopState, OutputId, WindowId, WorkspaceId};
use tilling_ipc::ResourceId;

use super::refusal::Refusal;

pub(super) fn window_id(window: WindowId) -> ResourceId {
    ResourceId {
        index: window.index(),
        generation: window.generation(),
    }
}

pub(super) fn workspace_id(workspace: WorkspaceId) -> ResourceId {
    ResourceId {
        index: workspace.index(),
        generation: workspace.generation(),
    }
}

pub(super) fn output_id(output: OutputId) -> ResourceId {
    ResourceId {
        index: output.index(),
        generation: output.generation(),
    }
}

pub(super) fn window(desktop: &DesktopState, id: ResourceId) -> Result<WindowId, Refusal> {
    desktop
        .windows()
        .map(|(window, _)| window)
        .find(|window| window_id(*window) == id)
        .ok_or_else(|| missing("window", id))
}

pub(super) fn workspace(desktop: &DesktopState, id: ResourceId) -> Result<WorkspaceId, Refusal> {
    desktop
        .workspaces()
        .map(|(workspace, _)| workspace)
        .find(|workspace| workspace_id(*workspace) == id)
        .ok_or_else(|| missing("workspace", id))
}

pub(super) fn output(desktop: &DesktopState, id: ResourceId) -> Result<OutputId, Refusal> {
    desktop
        .outputs()
        .map(|(output, _)| output)
        .find(|output| output_id(*output) == id)
        .ok_or_else(|| missing("output", id))
}

fn missing(kind: &str, id: ResourceId) -> Refusal {
    Refusal::NotFound(format!(
        "{kind} {}v{} does not exist",
        id.index, id.generation
    ))
}

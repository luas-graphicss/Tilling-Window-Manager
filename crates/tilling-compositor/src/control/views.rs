use tilling_core::layout::LayoutKind;
use tilling_core::{DesktopState, Output, OutputId, Window, WindowId, Workspace, WorkspaceId};
use tilling_ipc::{Layout, OutputInfo, WindowInfo, WindowMode, WorkspaceInfo};

use super::resources::{output_id, window_id, workspace_id};

pub(super) fn outputs(desktop: &DesktopState) -> Vec<OutputInfo> {
    desktop
        .outputs()
        .map(|(id, described)| output(id, described))
        .collect()
}

pub(super) fn output(id: OutputId, output: &Output) -> OutputInfo {
    let area = output.area();
    OutputInfo {
        id: output_id(id),
        name: output.name().to_owned(),
        x: area.x(),
        y: area.y(),
        width: area.width(),
        height: area.height(),
        scale: output.scale().factor(),
        workspace: workspace_id(output.workspace()),
    }
}

pub(super) fn workspaces(desktop: &DesktopState) -> Vec<WorkspaceInfo> {
    desktop
        .workspaces()
        .map(|(id, described)| workspace(desktop, id, described))
        .collect()
}

pub(super) fn workspace(
    desktop: &DesktopState,
    id: WorkspaceId,
    workspace: &Workspace,
) -> WorkspaceInfo {
    WorkspaceInfo {
        id: workspace_id(id),
        name: workspace.name().to_owned(),
        output: workspace.output().map(output_id),
        focused_window: workspace.focused_window().map(window_id),
        windows: workspace.windows().iter().copied().map(window_id).collect(),
        layout: layout(workspace.layout().kind()),
        active: desktop.active_workspace() == id,
    }
}

pub(super) fn windows(desktop: &DesktopState) -> Vec<WindowInfo> {
    desktop
        .windows()
        .map(|(id, described)| window(desktop, id, described))
        .collect()
}

pub(super) fn window(desktop: &DesktopState, id: WindowId, window: &Window) -> WindowInfo {
    let metadata = window.metadata();
    WindowInfo {
        id: window_id(id),
        app_id: metadata.app_id.clone(),
        title: metadata.title.clone(),
        workspace: workspace_id(window.workspace()),
        mode: mode(window.mode()),
        focused: desktop.focused_window() == Some(id),
    }
}

fn layout(kind: LayoutKind) -> Layout {
    match kind {
        LayoutKind::Bsp => Layout::Bsp,
        LayoutKind::MasterStack => Layout::MasterStack,
        LayoutKind::Monocle => Layout::Monocle,
    }
}

fn mode(mode: tilling_core::WindowMode) -> WindowMode {
    match mode {
        tilling_core::WindowMode::Tiled => WindowMode::Tiled,
        tilling_core::WindowMode::Floating(_) => WindowMode::Floating,
        tilling_core::WindowMode::Fullscreen(_) => WindowMode::Fullscreen,
    }
}

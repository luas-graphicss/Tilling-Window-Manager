use crate::geometry::{LogicalRect, Scale};
use crate::identity::{OutputId, WindowId, WorkspaceId};
use crate::layout::Direction;
use crate::window::{WindowMetadata, WindowMode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    AddOutput {
        name: String,
        area: LogicalRect,
        scale: Scale,
    },
    ConfigureOutput {
        output: OutputId,
        area: LogicalRect,
        scale: Scale,
    },
    RemoveOutput {
        output: OutputId,
    },
    CreateWorkspace {
        name: String,
    },
    SwitchWorkspace {
        workspace: WorkspaceId,
    },
    OpenWindow {
        metadata: WindowMetadata,
        workspace: Option<WorkspaceId>,
        transient_for: Option<WindowId>,
        mode: WindowMode,
    },
    CloseWindow {
        window: WindowId,
    },
    UpdateWindowMetadata {
        window: WindowId,
        metadata: WindowMetadata,
    },
    FocusWindow {
        window: WindowId,
    },
    FocusDirection {
        direction: Direction,
    },
    MoveWindow {
        window: WindowId,
        direction: Direction,
    },
    MoveWindowToWorkspace {
        window: WindowId,
        workspace: WorkspaceId,
    },
    ChangeWindowMode {
        window: WindowId,
        change: ModeChange,
    },
    ToggleSplit {
        window: WindowId,
    },
    ResizeSplit {
        window: WindowId,
        delta: i16,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeChange {
    Tile,
    Float(LogicalRect),
    EnterFullscreen,
    LeaveFullscreen,
}

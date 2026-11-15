use serde::{Deserialize, Serialize};

use crate::resource::{Direction, Layout, OutputChange, ResourceId, WindowMode};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request {
    pub request_id: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_revision: Option<u64>,
    #[serde(flatten)]
    pub call: Call,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "method", content = "params", rename_all = "snake_case")]
pub enum Call {
    GetVersion,
    GetOutputs,
    GetWorkspaces,
    GetWindows,
    GetConfig,
    GetDiagnostics,
    FocusWindow {
        window: ResourceId,
    },
    MoveWindow {
        window: ResourceId,
        direction: Direction,
    },
    SetLayout {
        workspace: ResourceId,
        layout: Layout,
    },
    ResizeWindow {
        window: ResourceId,
        delta: i16,
    },
    SetWindowMode {
        window: ResourceId,
        mode: WindowMode,
    },
    SwitchWorkspace {
        workspace: ResourceId,
    },
    ConfigureOutput {
        output: ResourceId,
        change: OutputChange,
    },
    ConfirmOutputChange,
    RevertOutputChange,
    ReloadConfig,
    RequestLock,
    Quit,
    Subscribe,
}

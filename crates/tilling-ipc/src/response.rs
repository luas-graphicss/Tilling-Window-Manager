use serde::{Deserialize, Serialize};

use crate::event::EventFrame;
use crate::resource::{ResourceId, WindowMode};
use crate::version::ProtocolVersion;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Response(Response),
    Event(EventFrame),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Response {
    pub request_id: u64,
    #[serde(flatten)]
    pub outcome: Outcome,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Outcome {
    Success { result: Reply },
    ValidationError { message: String },
    RevisionConflict { current_revision: u64 },
    NotFound { message: String },
    NotPermitted { message: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum Reply {
    Version(VersionInfo),
    Outputs(Vec<OutputInfo>),
    Workspaces(Vec<WorkspaceInfo>),
    Windows(Vec<WindowInfo>),
    Config(ConfigInfo),
    Diagnostics(DiagnosticsInfo),
    Accepted { revision: u64 },
    Subscribed(Snapshot),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionInfo {
    pub compositor: String,
    pub protocol: ProtocolVersion,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputInfo {
    pub id: ResourceId,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
    pub workspace: ResourceId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub id: ResourceId,
    pub name: String,
    pub output: Option<ResourceId>,
    pub focused_window: Option<ResourceId>,
    pub windows: Vec<ResourceId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowInfo {
    pub id: ResourceId,
    pub app_id: Option<String>,
    pub title: Option<String>,
    pub workspace: ResourceId,
    pub mode: WindowMode,
    pub focused: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigInfo {
    pub revision: u64,
    pub path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticsInfo {
    pub revision: u64,
    pub outputs: u32,
    pub workspaces: u32,
    pub windows: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub sequence: u64,
    pub revision: u64,
    pub outputs: Vec<OutputInfo>,
    pub workspaces: Vec<WorkspaceInfo>,
    pub windows: Vec<WindowInfo>,
}

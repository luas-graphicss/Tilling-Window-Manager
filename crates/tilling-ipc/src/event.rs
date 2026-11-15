use serde::{Deserialize, Serialize};

use crate::resource::ResourceId;
use crate::response::{OutputInfo, WindowInfo, WorkspaceInfo};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventFrame {
    pub sequence: u64,
    pub revision: u64,
    #[serde(flatten)]
    pub event: Event,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
pub enum Event {
    WindowOpened(WindowInfo),
    WindowChanged(WindowInfo),
    WindowClosed { window: ResourceId },
    FocusChanged { window: Option<ResourceId> },
    WorkspaceChanged(WorkspaceInfo),
    OutputChanged(OutputInfo),
    OutputRemoved { output: ResourceId },
    EventsDropped,
}

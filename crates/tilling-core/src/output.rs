pub(crate) mod adjacency;

use serde::Serialize;

use crate::geometry::{LogicalRect, Scale};
use crate::identity::WorkspaceId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Output {
    pub(crate) name: String,
    pub(crate) area: LogicalRect,
    pub(crate) scale: Scale,
    pub(crate) workspace: WorkspaceId,
}

impl Output {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn area(&self) -> LogicalRect {
        self.area
    }

    #[must_use]
    pub fn scale(&self) -> Scale {
        self.scale
    }

    #[must_use]
    pub fn workspace(&self) -> WorkspaceId {
        self.workspace
    }
}

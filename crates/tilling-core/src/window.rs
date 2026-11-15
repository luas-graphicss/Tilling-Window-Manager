use serde::Serialize;

use crate::geometry::LogicalRect;
use crate::identity::{WindowId, WorkspaceId};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct WindowMetadata {
    pub app_id: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum WindowMode {
    Tiled,
    Floating(LogicalRect),
    Fullscreen(RestoreMode),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RestoreMode {
    Tiled,
    Floating(LogicalRect),
}

impl WindowMode {
    #[must_use]
    pub const fn occupies_tile(self) -> bool {
        matches!(self, Self::Tiled | Self::Fullscreen(RestoreMode::Tiled))
    }

    pub(crate) const fn floating_area(self) -> Option<LogicalRect> {
        match self {
            Self::Floating(area) | Self::Fullscreen(RestoreMode::Floating(area)) => Some(area),
            Self::Tiled | Self::Fullscreen(RestoreMode::Tiled) => None,
        }
    }

    pub(crate) fn fitted_within(self, bounds: &LogicalRect) -> Self {
        match self {
            Self::Floating(area) => Self::Floating(area.fitted_within(bounds)),
            Self::Fullscreen(RestoreMode::Floating(area)) => {
                Self::Fullscreen(RestoreMode::Floating(area.fitted_within(bounds)))
            }
            Self::Tiled | Self::Fullscreen(RestoreMode::Tiled) => self,
        }
    }
}

impl From<RestoreMode> for WindowMode {
    fn from(restore: RestoreMode) -> Self {
        match restore {
            RestoreMode::Tiled => Self::Tiled,
            RestoreMode::Floating(area) => Self::Floating(area),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Window {
    pub(crate) metadata: WindowMetadata,
    pub(crate) workspace: WorkspaceId,
    pub(crate) mode: WindowMode,
    pub(crate) transient_for: Option<WindowId>,
}

impl Window {
    #[must_use]
    pub fn metadata(&self) -> &WindowMetadata {
        &self.metadata
    }

    #[must_use]
    pub fn workspace(&self) -> WorkspaceId {
        self.workspace
    }

    #[must_use]
    pub fn mode(&self) -> WindowMode {
        self.mode
    }

    #[must_use]
    pub fn transient_for(&self) -> Option<WindowId> {
        self.transient_for
    }
}

use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::compositor::with_states;
use smithay::wayland::shell::xdg::SurfaceCachedState;
use tilling_config::RuleMode;
use tilling_core::geometry::LogicalRect;
use tilling_core::{Command, RestoreMode, WindowId, WindowMetadata, WindowMode, WorkspaceId};

use super::metadata;
use super::registry::ManagedWindow;
use crate::geometry::centered;
use crate::runtime::Compositor;
use crate::scene::BORDER_WIDTH;

const DEFAULT_FLOATING_WIDTH: u32 = 640;
const DEFAULT_FLOATING_HEIGHT: u32 = 480;

#[derive(Debug, Clone)]
pub(crate) struct WindowPlan {
    pub(crate) metadata: WindowMetadata,
    pub(crate) workspace: Option<WorkspaceId>,
    pub(crate) transient_for: Option<WindowId>,
    pub(crate) mode: WindowMode,
}

impl WindowPlan {
    pub(crate) fn into_command(self) -> Command {
        Command::OpenWindow {
            metadata: self.metadata,
            workspace: self.workspace,
            transient_for: self.transient_for,
            mode: self.mode,
        }
    }
}

impl Compositor {
    pub(crate) fn plan_window(&self, managed: &ManagedWindow) -> WindowPlan {
        let surface = managed.toplevel.wl_surface();
        let metadata = metadata::read(surface);
        let rule = self.config.rules().resolve(&metadata);
        let workspace = rule
            .and_then(|outcome| outcome.workspace.as_deref())
            .and_then(|name| self.workspace_named(name));
        let transient_for = managed
            .toplevel
            .parent()
            .and_then(|parent| self.windows.id_of(&parent));
        let target = workspace.unwrap_or_else(|| self.desktop.active_workspace());
        let floating = || {
            self.floating_area(target, managed)
                .map_or(WindowMode::Tiled, WindowMode::Floating)
        };
        let mode = match rule.and_then(|outcome| outcome.mode) {
            Some(RuleMode::Tiled) => WindowMode::Tiled,
            Some(RuleMode::Floating) => floating(),
            Some(RuleMode::Fullscreen) => WindowMode::Fullscreen(RestoreMode::Tiled),
            None if managed.fullscreen_requested => WindowMode::Fullscreen(RestoreMode::Tiled),
            None if transient_for.is_some() || has_fixed_size(surface) => floating(),
            None => WindowMode::Tiled,
        };
        WindowPlan {
            metadata,
            workspace,
            transient_for,
            mode,
        }
    }

    pub(crate) fn floating_area(
        &self,
        workspace: WorkspaceId,
        managed: &ManagedWindow,
    ) -> Option<LogicalRect> {
        let size = managed.window.geometry().size;
        let width = positive(size.w).unwrap_or(DEFAULT_FLOATING_WIDTH);
        let height = positive(size.h).unwrap_or(DEFAULT_FLOATING_HEIGHT);
        centered(
            self.workspace_area(workspace)?,
            width.saturating_add(2 * BORDER_WIDTH),
            height.saturating_add(2 * BORDER_WIDTH),
        )
    }

    fn workspace_area(&self, workspace: WorkspaceId) -> Option<LogicalRect> {
        let output = match self.desktop.workspace(workspace)?.output() {
            Some(owner) => self.desktop.output(owner)?,
            None => self.desktop.outputs().next()?.1,
        };
        Some(output.area())
    }
}

fn positive(length: i32) -> Option<u32> {
    u32::try_from(length).ok().filter(|length| *length > 0)
}

fn has_fixed_size(surface: &WlSurface) -> bool {
    with_states(surface, |states| {
        let mut cached = states.cached_state.get::<SurfaceCachedState>();
        let limits = cached.current();
        limits.min_size.w > 0 && limits.min_size.h > 0 && limits.min_size == limits.max_size
    })
}

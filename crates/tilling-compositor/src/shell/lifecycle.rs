use smithay::backend::renderer::utils::with_renderer_surface_state;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::shell::xdg::ToplevelSurface;
use tilling_core::{Command, ModeChange, WindowId};

use super::configure::opened_window;
use super::metadata;
use super::registry::Lifecycle;
use crate::runtime::Compositor;

impl Compositor {
    pub(crate) fn toplevel_committed(&mut self, root: &WlSurface) {
        let Some(managed) = self.windows.get(root) else {
            return;
        };
        managed.window.on_commit();
        let lifecycle = managed.lifecycle;
        let attached = has_buffer(root);
        if attached && !managed.toplevel.ensure_configured() {
            return;
        }
        if !managed.toplevel.is_initial_configure_sent() {
            self.send_initial_configure(root);
            return;
        }
        match (lifecycle, attached) {
            (Lifecycle::Unmapped, true) => self.map_window(root),
            (Lifecycle::Mapped(window), false) => self.unmap_window(root, window),
            (Lifecycle::Mapped(window), true) => self.transactions.commit(window),
            (Lifecycle::Unmapped, false) => {}
        }
    }

    pub(crate) fn forget_toplevel(&mut self, surface: &WlSurface) {
        let Some(managed) = self.windows.remove(surface) else {
            return;
        };
        if let Lifecycle::Mapped(window) = managed.lifecycle {
            self.close_window(window);
        }
    }

    pub(crate) fn refresh_metadata(&mut self, surface: &WlSurface) {
        let Some(window) = self.windows.id_of(surface) else {
            return;
        };
        let command = Command::UpdateWindowMetadata {
            window,
            metadata: metadata::read(surface),
        };
        if let Err(error) = self.execute(command) {
            tracing::warn!(%error, %window, "update window metadata");
        }
    }

    pub(crate) fn set_fullscreen(&mut self, toplevel: &ToplevelSurface, fullscreen: bool) {
        let surface = toplevel.wl_surface();
        let Some(managed) = self.windows.get_mut(surface) else {
            return;
        };
        managed.fullscreen_requested = fullscreen;
        if let Lifecycle::Mapped(window) = managed.lifecycle {
            let change = if fullscreen {
                ModeChange::EnterFullscreen
            } else {
                ModeChange::LeaveFullscreen
            };
            if let Err(error) = self.execute(Command::ChangeWindowMode { window, change }) {
                tracing::warn!(%error, %window, "change fullscreen state");
            }
        }
        if toplevel.is_initial_configure_sent() {
            toplevel.send_configure();
        }
    }

    fn map_window(&mut self, surface: &WlSurface) {
        let Some(managed) = self.windows.get(surface) else {
            return;
        };
        let toplevel = managed.toplevel.clone();
        let command = self.plan_window(managed).into_command();
        let transition = match self.desktop.apply(command) {
            Ok(transition) => transition,
            Err(error) => {
                tracing::warn!(%error, "refuse to manage window");
                toplevel.send_close();
                return;
            }
        };
        let Some(window) = opened_window(&transition) else {
            return;
        };
        self.windows.mark_mapped(surface, window);
        if let Some(output) = self.outputs.primary() {
            output.enter(surface);
        }
        self.react(&transition);
    }

    fn unmap_window(&mut self, surface: &WlSurface, window: WindowId) {
        if let Some(managed) = self.windows.get(surface) {
            managed.toplevel.reset_initial_configure_sent();
        }
        self.windows.mark_unmapped(surface);
        self.close_window(window);
    }

    fn close_window(&mut self, window: WindowId) {
        if let Err(error) = self.execute(Command::CloseWindow { window }) {
            tracing::warn!(%error, %window, "close window");
        }
    }
}

fn has_buffer(surface: &WlSurface) -> bool {
    with_renderer_surface_state(surface, |state| state.buffer().is_some()).unwrap_or(false)
}

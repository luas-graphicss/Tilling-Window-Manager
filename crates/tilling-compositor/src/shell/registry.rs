use std::collections::{BTreeMap, HashMap};

use smithay::desktop::Window;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::shell::xdg::ToplevelSurface;
use tilling_core::WindowId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Lifecycle {
    Unmapped,
    Mapped(WindowId),
}

#[derive(Debug)]
pub(crate) struct ManagedWindow {
    pub(crate) window: Window,
    pub(crate) toplevel: ToplevelSurface,
    pub(crate) lifecycle: Lifecycle,
    pub(crate) fullscreen_requested: bool,
}

#[derive(Debug, Default)]
pub(crate) struct WindowRegistry {
    by_surface: HashMap<WlSurface, ManagedWindow>,
    by_id: BTreeMap<WindowId, WlSurface>,
}

impl WindowRegistry {
    pub(crate) fn insert(&mut self, window: Window, toplevel: ToplevelSurface) {
        let surface = toplevel.wl_surface().clone();
        let managed = ManagedWindow {
            window,
            toplevel,
            lifecycle: Lifecycle::Unmapped,
            fullscreen_requested: false,
        };
        self.by_surface.insert(surface, managed);
    }

    pub(crate) fn remove(&mut self, surface: &WlSurface) -> Option<ManagedWindow> {
        let managed = self.by_surface.remove(surface)?;
        if let Lifecycle::Mapped(id) = managed.lifecycle {
            self.by_id.remove(&id);
        }
        Some(managed)
    }

    pub(crate) fn get(&self, surface: &WlSurface) -> Option<&ManagedWindow> {
        self.by_surface.get(surface)
    }

    pub(crate) fn get_mut(&mut self, surface: &WlSurface) -> Option<&mut ManagedWindow> {
        self.by_surface.get_mut(surface)
    }

    pub(crate) fn by_id(&self, id: WindowId) -> Option<&ManagedWindow> {
        self.by_surface.get(self.by_id.get(&id)?)
    }

    pub(crate) fn id_of(&self, surface: &WlSurface) -> Option<WindowId> {
        match self.by_surface.get(surface)?.lifecycle {
            Lifecycle::Mapped(id) => Some(id),
            Lifecycle::Unmapped => None,
        }
    }

    pub(crate) fn surface_of(&self, id: WindowId) -> Option<&WlSurface> {
        self.by_id.get(&id)
    }

    pub(crate) fn mark_mapped(&mut self, surface: &WlSurface, id: WindowId) {
        if let Some(managed) = self.by_surface.get_mut(surface) {
            managed.lifecycle = Lifecycle::Mapped(id);
            self.by_id.insert(id, surface.clone());
        }
    }

    pub(crate) fn mark_unmapped(&mut self, surface: &WlSurface) {
        let Some(managed) = self.by_surface.get_mut(surface) else {
            return;
        };
        if let Lifecycle::Mapped(id) = managed.lifecycle {
            self.by_id.remove(&id);
        }
        managed.lifecycle = Lifecycle::Unmapped;
    }

    pub(crate) fn len(&self) -> usize {
        self.by_surface.len()
    }

    pub(crate) fn mapped_len(&self) -> usize {
        self.by_id.len()
    }
}

use smithay::desktop::{Window, WindowSurfaceType};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Logical, Point};
use smithay::wayland::shell::wlr_layer::Layer;
use tilling_core::WindowId;
use tilling_core::geometry::LogicalRect;

use super::registry::ManagedWindow;
use crate::geometry::rectangle;
use crate::runtime::Compositor;
use crate::scene::{SceneLayer, SceneWindow};

pub(crate) type SurfaceHit = (WlSurface, Point<f64, Logical>);

enum WindowHit {
    Surface(SurfaceHit),
    Frame,
    Outside,
}

pub(crate) fn surface_origin(placed: &SceneWindow, window: &Window) -> Point<i32, Logical> {
    rectangle(placed.content).loc - window.geometry().loc
}

impl Compositor {
    pub(crate) fn surface_under(&self, location: Point<f64, Logical>) -> Option<SurfaceHit> {
        if self.lock.is_engaged() {
            return self.lock_surface_under(location);
        }
        let above_windows = self
            .layer_surface_under(location, &[Layer::Overlay])
            .or_else(|| self.popup_under(location))
            .or_else(|| self.panel_under(location));
        if above_windows.is_some() {
            return above_windows;
        }
        match self.window_under(location) {
            WindowHit::Surface(hit) => Some(hit),
            WindowHit::Frame => None,
            WindowHit::Outside => {
                self.layer_surface_under(location, &[Layer::Bottom, Layer::Background])
            }
        }
    }

    pub(crate) fn window_at(&self, location: Point<f64, Logical>) -> Option<WindowId> {
        self.scene
            .windows()
            .iter()
            .rev()
            .find(|placed| covers(placed.frame, location))
            .map(|placed| placed.id)
    }

    fn panel_under(&self, location: Point<f64, Logical>) -> Option<SurfaceHit> {
        let below_fullscreen =
            self.scene.windows().iter().any(|placed| {
                placed.layer == SceneLayer::Fullscreen && covers(placed.frame, location)
            });
        if below_fullscreen {
            return None;
        }
        self.layer_surface_under(location, &[Layer::Top])
    }

    fn popup_under(&self, location: Point<f64, Logical>) -> Option<SurfaceHit> {
        self.placed_windows()
            .find_map(|(placed, managed)| hit(placed, managed, location, WindowSurfaceType::POPUP))
    }

    fn window_under(&self, location: Point<f64, Logical>) -> WindowHit {
        let Some((placed, managed)) = self
            .placed_windows()
            .find(|(placed, _)| covers(placed.frame, location))
        else {
            return WindowHit::Outside;
        };
        if !covers(placed.content, location) {
            return WindowHit::Frame;
        }
        let content = WindowSurfaceType::TOPLEVEL | WindowSurfaceType::SUBSURFACE;
        hit(placed, managed, location, content).map_or(WindowHit::Frame, WindowHit::Surface)
    }

    fn placed_windows(&self) -> impl Iterator<Item = (&SceneWindow, &ManagedWindow)> {
        self.scene
            .windows()
            .iter()
            .rev()
            .filter_map(|placed| Some((placed, self.windows.by_id(placed.id)?)))
    }
}

fn hit(
    placed: &SceneWindow,
    managed: &ManagedWindow,
    location: Point<f64, Logical>,
    kind: WindowSurfaceType,
) -> Option<SurfaceHit> {
    let origin = surface_origin(placed, &managed.window);
    managed
        .window
        .surface_under(location - origin.to_f64(), kind)
        .map(|(surface, offset)| (surface, (offset + origin).to_f64()))
}

fn covers(area: LogicalRect, location: Point<f64, Logical>) -> bool {
    rectangle(area).to_f64().contains(location)
}

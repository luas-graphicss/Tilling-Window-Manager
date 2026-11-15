use smithay::desktop::{Window, WindowSurfaceType};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Logical, Point};
use tilling_core::WindowId;
use tilling_core::geometry::LogicalRect;

use crate::geometry::rectangle;
use crate::runtime::Compositor;
use crate::scene::SceneWindow;

pub(crate) type SurfaceHit = (WlSurface, Point<f64, Logical>);

pub(crate) fn surface_origin(placed: &SceneWindow, window: &Window) -> Point<i32, Logical> {
    rectangle(placed.content).loc - window.geometry().loc
}

impl Compositor {
    pub(crate) fn surface_under(&self, location: Point<f64, Logical>) -> Option<SurfaceHit> {
        for placed in self.scene.windows().iter().rev() {
            let Some(managed) = self.windows.by_id(placed.id) else {
                continue;
            };
            let origin = surface_origin(placed, &managed.window);
            let hit = |kind: WindowSurfaceType| {
                managed
                    .window
                    .surface_under(location - origin.to_f64(), kind)
                    .map(|(surface, offset)| (surface, (offset + origin).to_f64()))
            };
            if let Some(popup) = hit(WindowSurfaceType::POPUP) {
                return Some(popup);
            }
            if !covers(placed.frame, location) {
                continue;
            }
            if !covers(placed.content, location) {
                return None;
            }
            return hit(WindowSurfaceType::TOPLEVEL | WindowSurfaceType::SUBSURFACE);
        }
        None
    }

    pub(crate) fn window_at(&self, location: Point<f64, Logical>) -> Option<WindowId> {
        self.scene
            .windows()
            .iter()
            .rev()
            .find(|placed| covers(placed.frame, location))
            .map(|placed| placed.id)
    }
}

fn covers(area: LogicalRect, location: Point<f64, Logical>) -> bool {
    rectangle(area).to_f64().contains(location)
}

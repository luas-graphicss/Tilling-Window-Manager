use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::element::surface::render_elements_from_surface_tree;
use smithay::backend::renderer::{ImportAll, ImportMem, Renderer};
use smithay::input::pointer::{CursorImageStatus, CursorImageSurfaceData};
use smithay::reexports::wayland_server::Resource;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Logical, Point, Scale};
use smithay::wayland::compositor::with_states;

use super::elements::SceneElement;
use crate::runtime::Compositor;

impl Compositor {
    pub(crate) fn cursor_surface(&self) -> Option<&WlSurface> {
        match &self.cursor {
            CursorImageStatus::Surface(surface) if surface.is_alive() => Some(surface),
            CursorImageStatus::Surface(_)
            | CursorImageStatus::Named(_)
            | CursorImageStatus::Hidden => None,
        }
    }
}

pub(super) fn cursor_elements<R>(
    compositor: &Compositor,
    renderer: &mut R,
    output_origin: Point<i32, Logical>,
    scale: Scale<f64>,
) -> Vec<SceneElement<R>>
where
    R: Renderer + ImportAll + ImportMem,
    R::TextureId: Clone + 'static,
{
    let Some(surface) = compositor.cursor_surface() else {
        return Vec::new();
    };
    let Some(pointer) = compositor.seat.get_pointer() else {
        return Vec::new();
    };
    let tip = pointer.current_location() - output_origin.to_f64() - hotspot(surface).to_f64();
    render_elements_from_surface_tree(
        renderer,
        surface,
        tip.to_physical_precise_round(scale),
        scale,
        1.0,
        Kind::Cursor,
    )
}

fn hotspot(surface: &WlSurface) -> Point<i32, Logical> {
    with_states(surface, |states| {
        states
            .data_map
            .get::<CursorImageSurfaceData>()
            .and_then(|attributes| attributes.lock().ok())
            .map_or_else(Point::default, |attributes| attributes.hotspot)
    })
}

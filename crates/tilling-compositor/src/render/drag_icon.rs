use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::element::surface::render_elements_from_surface_tree;
use smithay::backend::renderer::{ImportAll, ImportMem, Renderer};
use smithay::utils::{IsAlive, Logical, Point, Scale};

use super::elements::SceneElement;
use crate::runtime::Compositor;

pub(super) fn drag_icon_elements<R>(
    compositor: &Compositor,
    renderer: &mut R,
    output_origin: Point<i32, Logical>,
    scale: Scale<f64>,
) -> Vec<SceneElement<R>>
where
    R: Renderer + ImportAll + ImportMem,
    R::TextureId: Clone + 'static,
{
    let dragged = compositor.drag_icon.as_ref().filter(|icon| icon.alive());
    let (Some(icon), Some(pointer)) = (dragged, compositor.seat.get_pointer()) else {
        return Vec::new();
    };
    let tip = pointer.current_location() - output_origin.to_f64();
    render_elements_from_surface_tree(
        renderer,
        icon,
        tip.to_physical_precise_round(scale),
        scale,
        1.0,
        Kind::Unspecified,
    )
}

#[cfg(feature = "drm")]
use smithay::backend::renderer::element::memory::MemoryRenderBufferRenderElement;
use smithay::backend::renderer::element::solid::SolidColorRenderElement;
use smithay::backend::renderer::element::surface::{
    WaylandSurfaceRenderElement, render_elements_from_surface_tree,
};
use smithay::backend::renderer::element::utils::CropRenderElement;
use smithay::backend::renderer::element::{Kind, render_elements};
use smithay::backend::renderer::{ImportAll, ImportMem, Renderer};
use smithay::desktop::PopupManager;
use smithay::utils::{Logical, Point, Scale};

use crate::geometry::rectangle;
use crate::scene::SceneWindow;
use crate::shell::{ManagedWindow, surface_origin};

render_elements! {
    pub(crate) SceneElement<R> where R: ImportAll + ImportMem;
    Surface = WaylandSurfaceRenderElement<R>,
    Clipped = CropRenderElement<WaylandSurfaceRenderElement<R>>,
    Border = SolidColorRenderElement,
    #[cfg(feature = "drm")]
    Sprite = MemoryRenderBufferRenderElement<R>,
}

pub(super) fn content_elements<R>(
    renderer: &mut R,
    placed: &SceneWindow,
    managed: &ManagedWindow,
    output_origin: Point<i32, Logical>,
    scale: Scale<f64>,
) -> Vec<SceneElement<R>>
where
    R: Renderer + ImportAll + ImportMem,
    R::TextureId: Clone + 'static,
{
    let location = surface_origin(placed, &managed.window) - output_origin;
    let mut visible = rectangle(placed.content);
    visible.loc -= output_origin;
    let visible = visible.to_physical_precise_round(scale);
    let surfaces: Vec<WaylandSurfaceRenderElement<R>> = render_elements_from_surface_tree(
        renderer,
        managed.toplevel.wl_surface(),
        location.to_physical_precise_round(scale),
        scale,
        1.0,
        Kind::Unspecified,
    );
    surfaces
        .into_iter()
        .filter_map(|surface| CropRenderElement::from_element(surface, scale, visible))
        .map(SceneElement::Clipped)
        .collect()
}

pub(super) fn popup_elements<R>(
    renderer: &mut R,
    placed: &SceneWindow,
    managed: &ManagedWindow,
    output_origin: Point<i32, Logical>,
    scale: Scale<f64>,
) -> Vec<SceneElement<R>>
where
    R: Renderer + ImportAll + ImportMem,
    R::TextureId: Clone + 'static,
{
    let window_origin = rectangle(placed.content).loc - output_origin;
    PopupManager::popups_for_surface(managed.toplevel.wl_surface())
        .flat_map(|(popup, offset)| {
            let location = window_origin + offset - popup.geometry().loc;
            render_elements_from_surface_tree(
                renderer,
                popup.wl_surface(),
                location.to_physical_precise_round(scale),
                scale,
                1.0,
                Kind::Unspecified,
            )
        })
        .collect()
}

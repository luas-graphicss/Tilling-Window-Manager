use smithay::backend::renderer::element::AsRenderElements;
use smithay::backend::renderer::{ImportAll, ImportMem, Renderer};
use smithay::desktop::layer_map_for_output;
use smithay::output::Output;
use smithay::utils::Scale;
use smithay::wayland::shell::wlr_layer::Layer;

use super::elements::SceneElement;

pub(super) fn layer_elements<R>(
    renderer: &mut R,
    output: &Output,
    level: Layer,
    scale: Scale<f64>,
) -> Vec<SceneElement<R>>
where
    R: Renderer + ImportAll + ImportMem,
    R::TextureId: Clone + 'static,
{
    let map = layer_map_for_output(output);
    let mut elements = Vec::new();
    for layer in map.layers_on(level).rev() {
        let Some(geometry) = map.layer_geometry(layer) else {
            continue;
        };
        let position = geometry.loc - layer.bbox().loc;
        elements.extend(AsRenderElements::<R>::render_elements::<SceneElement<R>>(
            layer,
            renderer,
            position.to_physical_precise_round(scale),
            scale,
            1.0,
        ));
    }
    elements
}

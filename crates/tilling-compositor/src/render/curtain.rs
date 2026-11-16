use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::element::solid::{SolidColorBuffer, SolidColorRenderElement};
use smithay::backend::renderer::element::surface::render_elements_from_surface_tree;
use smithay::backend::renderer::{Color32F, ImportAll, ImportMem, Renderer};
use smithay::output::Output;
use smithay::utils::{Physical, Point, Scale};

use super::elements::SceneElement;
use crate::geometry::logical_size;
use crate::runtime::Compositor;

const CURTAIN: Color32F = Color32F::new(0.0, 0.0, 0.0, 1.0);

#[derive(Debug)]
pub(super) struct Curtain {
    cover: SolidColorBuffer,
}

impl Default for Curtain {
    fn default() -> Self {
        Self {
            cover: SolidColorBuffer::new((0, 0), CURTAIN),
        }
    }
}

impl Curtain {
    pub(super) fn elements<R>(
        &mut self,
        compositor: &Compositor,
        renderer: &mut R,
        output: &Output,
        scale: Scale<f64>,
    ) -> Vec<SceneElement<R>>
    where
        R: Renderer + ImportAll + ImportMem,
        R::TextureId: Clone + 'static,
    {
        let corner = Point::<i32, Physical>::default();
        let mut elements = Vec::new();
        if let Some(surface) = compositor.lock.surface_on(output) {
            elements.extend(render_elements_from_surface_tree(
                renderer,
                surface.wl_surface(),
                corner,
                scale,
                1.0,
                Kind::Unspecified,
            ));
        }
        self.cover
            .update(logical_size(output).unwrap_or_default(), CURTAIN);
        let cover = SolidColorRenderElement::from_buffer(
            &self.cover,
            corner,
            scale,
            1.0,
            Kind::Unspecified,
        );
        elements.push(SceneElement::from(cover));
        elements
    }
}

use smithay::backend::renderer::damage::{Error as DamageError, OutputDamageTracker};
use smithay::backend::renderer::{Color32F, ImportAll, ImportMem, Renderer};
use smithay::output::Output;
use smithay::utils::{Physical, Rectangle, Scale};

use super::borders::BorderCache;
use super::cursor::cursor_elements;
use super::elements::{SceneElement, content_elements, popup_elements};
use crate::runtime::Compositor;

pub(crate) const BACKGROUND: Color32F = Color32F::new(0.07, 0.08, 0.10, 1.0);

pub(crate) type Damage = Vec<Rectangle<i32, Physical>>;

#[derive(Debug)]
pub(crate) struct SceneView {
    output: Output,
    borders: BorderCache,
}

#[derive(Debug)]
pub(crate) struct Painter {
    view: SceneView,
    damage: OutputDamageTracker,
}

impl SceneView {
    pub(crate) fn new(output: &Output) -> Self {
        Self {
            output: output.clone(),
            borders: BorderCache::default(),
        }
    }

    pub(crate) fn elements<R>(
        &mut self,
        compositor: &Compositor,
        renderer: &mut R,
    ) -> Vec<SceneElement<R>>
    where
        R: Renderer + ImportAll + ImportMem,
        R::TextureId: Clone + 'static,
    {
        let scale = Scale::from(self.output.current_scale().fractional_scale());
        let origin = self.output.current_location();
        let shown = compositor.outputs.id_of(&self.output);
        let windows: Vec<_> = compositor
            .scene
            .windows()
            .iter()
            .rev()
            .filter(|placed| Some(placed.output) == shown)
            .filter_map(|placed| Some((placed, compositor.windows.by_id(placed.id)?)))
            .collect();
        let mut elements = cursor_elements(compositor, renderer, origin, scale);
        for (placed, managed) in &windows {
            elements.extend(popup_elements(renderer, placed, managed, origin, scale));
        }
        for (placed, managed) in &windows {
            elements.extend(content_elements(renderer, placed, managed, origin, scale));
            let borders = self.borders.elements(placed, origin, scale);
            elements.extend(borders.into_iter().map(SceneElement::Border));
        }
        self.borders.forget_windows_outside(&compositor.scene);
        elements
    }
}

impl Painter {
    pub(crate) fn new(output: &Output) -> Self {
        Self {
            view: SceneView::new(output),
            damage: OutputDamageTracker::from_output(output),
        }
    }

    pub(crate) fn paint<R>(
        &mut self,
        compositor: &Compositor,
        renderer: &mut R,
        framebuffer: &mut R::Framebuffer<'_>,
        buffer_age: usize,
    ) -> Result<Option<Damage>, DamageError<R::Error>>
    where
        R: Renderer + ImportAll + ImportMem,
        R::TextureId: Clone + 'static,
    {
        let elements = self.view.elements(compositor, renderer);
        let painted =
            self.damage
                .render_output(renderer, framebuffer, buffer_age, &elements, BACKGROUND)?;
        Ok(painted.damage.cloned())
    }
}

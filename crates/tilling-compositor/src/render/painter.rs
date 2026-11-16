use smithay::backend::renderer::damage::{Error as DamageError, OutputDamageTracker};
use smithay::backend::renderer::{Color32F, ImportAll, ImportMem, Renderer};
use smithay::output::Output;
use smithay::utils::{Physical, Rectangle, Scale};
use smithay::wayland::shell::wlr_layer::Layer;

use super::borders::BorderCache;
use super::cursor::cursor_elements;
use super::curtain::Curtain;
use super::drag_icon::drag_icon_elements;
use super::elements::{SceneElement, content_elements, popup_elements};
use super::framebuffer::FramebufferOrigin;
use super::layers::layer_elements;
use crate::runtime::Compositor;
use crate::scene::SceneLayer;

pub(crate) const BACKGROUND: Color32F = Color32F::new(0.07, 0.08, 0.10, 1.0);

pub(crate) type Damage = Vec<Rectangle<i32, Physical>>;

#[derive(Debug)]
pub(crate) struct SceneView {
    output: Output,
    borders: BorderCache,
    curtain: Curtain,
}

#[derive(Debug)]
pub(crate) struct Painter {
    view: SceneView,
    origin: FramebufferOrigin,
    damage: OutputDamageTracker,
}

impl SceneView {
    pub(crate) fn new(output: &Output) -> Self {
        Self {
            output: output.clone(),
            borders: BorderCache::default(),
            curtain: Curtain::default(),
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
        if compositor.lock.is_engaged() {
            let curtain = &mut self.curtain;
            elements.extend(curtain.elements(compositor, renderer, &self.output, scale));
            return elements;
        }
        elements.extend(drag_icon_elements(compositor, renderer, origin, scale));
        elements.extend(layer_elements(
            renderer,
            &self.output,
            Layer::Overlay,
            scale,
        ));
        for (placed, managed) in &windows {
            elements.extend(popup_elements(renderer, placed, managed, origin, scale));
        }
        let (fullscreen, beside_panels): (Vec<_>, Vec<_>) = windows
            .iter()
            .partition(|(placed, _)| placed.layer == SceneLayer::Fullscreen);
        let panels = layer_elements(renderer, &self.output, Layer::Top, scale);
        for (placed, managed) in fullscreen {
            elements.extend(content_elements(renderer, placed, managed, origin, scale));
        }
        elements.extend(panels);
        for (placed, managed) in beside_panels {
            elements.extend(content_elements(renderer, placed, managed, origin, scale));
            let borders = self.borders.elements(placed, origin, scale);
            elements.extend(borders.into_iter().map(SceneElement::Border));
        }
        for level in [Layer::Bottom, Layer::Background] {
            elements.extend(layer_elements(renderer, &self.output, level, scale));
        }
        self.borders.forget_windows_outside(&compositor.scene);
        elements
    }
}

impl Painter {
    pub(crate) fn new(output: &Output, origin: FramebufferOrigin) -> Self {
        Self {
            view: SceneView::new(output),
            origin,
            damage: OutputDamageTracker::from_mode_source(origin.mode_source(output)),
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
        self.follow_output();
        let elements = self.view.elements(compositor, renderer);
        let painted =
            self.damage
                .render_output(renderer, framebuffer, buffer_age, &elements, BACKGROUND)?;
        Ok(painted.damage.cloned())
    }

    fn follow_output(&mut self) {
        let wanted = self.origin.mode_source(&self.view.output);
        if *self.damage.mode() != wanted {
            self.damage = OutputDamageTracker::from_mode_source(wanted);
        }
    }
}

use smithay::backend::drm::output::DrmOutputRenderElements;
use smithay::output::Mode;

use super::{RenderError, RenderTarget, Surface};
use crate::backend::drm::DrmRenderer;
use crate::render::SceneElement;
use crate::runtime::Compositor;

const NOT_OFFERED: &str = "the connector does not offer it";

impl Surface {
    pub(in crate::backend::drm) fn follow_mode(
        &mut self,
        target: &mut RenderTarget<'_>,
        compositor: &mut Compositor,
    ) {
        let wanted = self
            .output
            .current_mode()
            .filter(|mode| *mode != self.programmed);
        let Some(wanted) = wanted else {
            return;
        };
        let output = self.output.name();
        match self.program(wanted, target) {
            Ok(()) => {
                let (width, height) = (wanted.size.w, wanted.size.h);
                tracing::info!(output, width, height, "display mode changed");
                self.programmed = wanted;
                self.cancel_throttle(compositor);
                self.clock.reset();
                compositor.demand_frame();
            }
            Err(error) => {
                tracing::warn!(%error, output, "keep the previous display mode");
                compositor.keep_output_mode(&self.output, self.programmed);
            }
        }
    }

    fn program(&mut self, wanted: Mode, target: &mut RenderTarget<'_>) -> Result<(), RenderError> {
        let mode = self
            .modes
            .iter()
            .copied()
            .find(|offered| Mode::from(*offered) == wanted)
            .ok_or_else(|| RenderError::Mode(NOT_OFFERED.to_owned()))?;
        let mut renderer = target.renderer(self.kms.format())?;
        let other_outputs =
            DrmOutputRenderElements::<DrmRenderer<'_>, SceneElement<DrmRenderer<'_>>>::default();
        self.kms
            .use_mode(mode, &mut renderer, &other_outputs)
            .map_err(|error| RenderError::Mode(error.to_string()))
    }
}

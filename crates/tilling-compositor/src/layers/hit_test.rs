use smithay::desktop::{WindowSurfaceType, layer_map_for_output};
use smithay::output::Output;
use smithay::utils::{Logical, Point};
use smithay::wayland::shell::wlr_layer::Layer;

use crate::geometry::rectangle;
use crate::runtime::Compositor;
use crate::shell::SurfaceHit;

impl Compositor {
    pub(crate) fn layer_surface_under(
        &self,
        location: Point<f64, Logical>,
        levels: &[Layer],
    ) -> Option<SurfaceHit> {
        let output = self.output_at(location)?;
        let origin = output.current_location();
        let map = layer_map_for_output(output);
        levels.iter().find_map(|level| {
            map.layers_on(*level).rev().find_map(|layer| {
                let position = map.layer_geometry(layer)?.loc - layer.bbox().loc + origin;
                let (surface, offset) =
                    layer.surface_under(location - position.to_f64(), WindowSurfaceType::ALL)?;
                Some((surface, (offset + position).to_f64()))
            })
        })
    }

    pub(crate) fn output_at(&self, location: Point<f64, Logical>) -> Option<&Output> {
        self.desktop
            .outputs()
            .find(|(_, output)| rectangle(output.area()).to_f64().contains(location))
            .and_then(|(id, _)| self.outputs.by_id(id))
    }
}

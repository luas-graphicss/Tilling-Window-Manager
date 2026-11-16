use smithay::desktop::{LayerSurface, WindowSurfaceType, layer_map_for_output};
use smithay::output::Output;
use smithay::reexports::wayland_server::protocol::wl_output::WlOutput;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::compositor::with_states;
use smithay::wayland::fractional_scale::with_fractional_scale;
use smithay::wayland::shell::wlr_layer::{LayerSurface as LayerRole, LayerSurfaceData};

use crate::runtime::Compositor;

impl Compositor {
    pub(crate) fn admit_layer(
        &mut self,
        role: LayerRole,
        requested: Option<&WlOutput>,
        namespace: String,
    ) {
        let output = requested
            .and_then(Output::from_resource)
            .or_else(|| self.active_output().cloned());
        let Some(output) = output else {
            role.send_close();
            return;
        };
        let layer = LayerSurface::new(role, namespace);
        let mapped = layer_map_for_output(&output).map_layer(&layer);
        if let Err(error) = mapped {
            tracing::warn!(%error, "refuse layer surface");
            layer.layer_surface().send_close();
        }
    }

    pub(crate) fn layer_committed(&mut self, surface: &WlSurface) {
        let Some((output, layer)) = self.layer_on_output(surface) else {
            return;
        };
        layer_map_for_output(&output).arrange();
        if !initial_configure_sent(surface) {
            layer.layer_surface().send_configure();
        }
        announce_output(surface, &output);
        self.layers_changed(&output);
    }

    pub(crate) fn retire_layer(&mut self, role: &LayerRole) {
        let owner = self.outputs.iter().find_map(|(_, output)| {
            let map = layer_map_for_output(output);
            let layer = map.layers().find(|layer| layer.layer_surface() == role)?;
            Some((output.clone(), layer.clone()))
        });
        let Some((output, layer)) = owner else {
            return;
        };
        layer_map_for_output(&output).unmap_layer(&layer);
        self.layers_changed(&output);
    }

    pub(crate) fn close_layers(output: &Output) {
        for layer in layer_map_for_output(output).layers() {
            layer.layer_surface().send_close();
        }
    }

    pub(crate) fn layer_on_output(&self, surface: &WlSurface) -> Option<(Output, LayerSurface)> {
        self.outputs.iter().find_map(|(_, output)| {
            let map = layer_map_for_output(output);
            let layer = map.layer_for_surface(surface, WindowSurfaceType::TOPLEVEL)?;
            Some((output.clone(), layer.clone()))
        })
    }

    pub(crate) fn layer_count(&self) -> usize {
        self.outputs
            .iter()
            .map(|(_, output)| layer_map_for_output(output).len())
            .sum()
    }

    fn layers_changed(&mut self, output: &Output) {
        self.reserve_layer_edges(output);
        self.sync_keyboard_focus();
        self.demand_frame();
    }
}

fn initial_configure_sent(surface: &WlSurface) -> bool {
    with_states(surface, |states| {
        states
            .data_map
            .get::<LayerSurfaceData>()
            .and_then(|attributes| attributes.lock().ok())
            .is_some_and(|attributes| attributes.initial_configure_sent)
    })
}

fn announce_output(surface: &WlSurface, output: &Output) {
    output.enter(surface);
    let scale = output.current_scale().fractional_scale();
    with_states(surface, |states| {
        with_fractional_scale(states, |fractional| {
            fractional.set_preferred_scale(scale);
        });
    });
}

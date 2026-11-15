use smithay::desktop::layer_map_for_output;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::IsAlive;
use smithay::wayland::shell::wlr_layer::{KeyboardInteractivity, Layer};

use crate::runtime::Compositor;

const LAYERS_ABOVE_WINDOWS: [Layer; 2] = [Layer::Overlay, Layer::Top];

impl Compositor {
    pub(crate) fn layer_keyboard_target(&self) -> Option<WlSurface> {
        self.exclusive_layer().or_else(|| self.chosen_layer())
    }

    pub(crate) fn choose_layer_at_press(&mut self, pressed: Option<&WlSurface>) {
        self.layer_focus = pressed
            .and_then(|surface| self.layer_on_output(surface))
            .filter(|(_, layer)| layer.can_receive_keyboard_focus())
            .map(|(_, layer)| layer.wl_surface().clone());
    }

    fn exclusive_layer(&self) -> Option<WlSurface> {
        LAYERS_ABOVE_WINDOWS.into_iter().find_map(|level| {
            self.outputs.iter().find_map(|(_, output)| {
                let map = layer_map_for_output(output);
                let demanding = map.layers_on(level).rev().find(|layer| {
                    layer.cached_state().keyboard_interactivity == KeyboardInteractivity::Exclusive
                })?;
                Some(demanding.wl_surface().clone())
            })
        })
    }

    fn chosen_layer(&self) -> Option<WlSurface> {
        let chosen = self.layer_focus.as_ref()?;
        let mapped = chosen.alive() && self.layer_on_output(chosen).is_some();
        mapped.then(|| chosen.clone())
    }
}

use std::time::Duration;

use smithay::reexports::wayland_server::Resource;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::xdg_activation::{
    XdgActivationHandler, XdgActivationState, XdgActivationToken, XdgActivationTokenData,
};
use tilling_core::Command;

use super::root_of;
use crate::runtime::Compositor;

const TOKEN_LIFETIME: Duration = Duration::from_secs(10);

impl XdgActivationHandler for Compositor {
    fn activation_state(&mut self) -> &mut XdgActivationState {
        &mut self.protocols.activation
    }

    fn token_created(&mut self, _token: XdgActivationToken, data: XdgActivationTokenData) -> bool {
        self.protocols
            .activation
            .retain_tokens(|_, issued| is_fresh(issued));
        self.requested_by_the_focused_client(&data)
    }

    fn request_activation(
        &mut self,
        token: XdgActivationToken,
        token_data: XdgActivationTokenData,
        surface: WlSurface,
    ) {
        self.protocols.activation.remove_token(&token);
        if !is_fresh(&token_data) || self.lock.is_engaged() {
            return;
        }
        if let Some(window) = self.windows.id_of(&root_of(&surface)) {
            self.run(Command::FocusWindow { window });
        }
    }
}

impl Compositor {
    fn requested_by_the_focused_client(&self, data: &XdgActivationTokenData) -> bool {
        let Some((serial, _)) = data.serial else {
            return false;
        };
        let focused = self
            .seat
            .get_keyboard()
            .and_then(|keyboard| keyboard.current_focus())
            .and_then(|surface| surface.client())
            .map(|client| client.id());
        self.input_serials.contains(serial) && focused.is_some() && focused == data.client_id
    }
}

fn is_fresh(data: &XdgActivationTokenData) -> bool {
    data.timestamp.elapsed() < TOKEN_LIFETIME
}

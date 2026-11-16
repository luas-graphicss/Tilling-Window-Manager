use wayland_client::{Connection, Dispatch, QueueHandle, delegate_noop};
use wayland_protocols::xdg::activation::v1::client::xdg_activation_token_v1::{
    self, XdgActivationTokenV1,
};
use wayland_protocols::xdg::activation::v1::client::xdg_activation_v1::XdgActivationV1;

use crate::client::{Client, ToplevelId};
use crate::error::ClientError;
use crate::state::ClientState;

delegate_noop!(ClientState: XdgActivationV1);

impl Dispatch<XdgActivationTokenV1, ()> for ClientState {
    fn event(
        state: &mut Self,
        _request: &XdgActivationTokenV1,
        event: xdg_activation_token_v1::Event,
        (): &(),
        _connection: &Connection,
        _queue: &QueueHandle<Self>,
    ) {
        if let xdg_activation_token_v1::Event::Done { token } = event {
            state.activation_tokens.push(token);
        }
    }
}

impl Client {
    pub fn request_activation_token(&mut self, serial: Option<u32>) -> Result<String, ClientError> {
        let activation: XdgActivationV1 = self.bind(1..=1, "xdg_activation_v1")?;
        let request = activation.get_activation_token(&self.queue.handle(), ());
        if let Some(serial) = serial {
            request.set_serial(serial, &self.seat);
        }
        request.commit();
        let issued = self.state.activation_tokens.len();
        let token = self.wait_for("an activation token", |state| {
            state.activation_tokens.get(issued).cloned()
        })?;
        request.destroy();
        Ok(token)
    }

    pub fn activate(&mut self, token: &str, toplevel: ToplevelId) -> Result<(), ClientError> {
        let activation: XdgActivationV1 = self.bind(1..=1, "xdg_activation_v1")?;
        activation.activate(token.to_owned(), self.surface_of(toplevel)?);
        self.roundtrip()
    }
}

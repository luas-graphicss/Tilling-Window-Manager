use wayland_client::delegate_noop;
use wayland_client::protocol::wl_region::WlRegion;

use crate::client::{Client, ToplevelId};
use crate::error::ClientError;
use crate::state::ClientState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputRegion {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

delegate_noop!(ClientState: WlRegion);

impl Client {
    pub fn set_input_region(
        &mut self,
        toplevel: ToplevelId,
        area: InputRegion,
    ) -> Result<(), ClientError> {
        let surface = self.surface_of(toplevel)?;
        let region = self.compositor.create_region(&self.queue.handle(), ());
        region.add(area.x, area.y, area.width, area.height);
        surface.set_input_region(Some(&region));
        surface.commit();
        region.destroy();
        self.roundtrip()
    }
}

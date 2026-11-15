use wayland_client::delegate_noop;
use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::wl_subcompositor::WlSubcompositor;
use wayland_client::protocol::wl_subsurface::WlSubsurface;
use wayland_client::protocol::wl_surface::WlSurface;

use crate::buffer;
use crate::client::{Client, ToplevelId};
use crate::error::ClientError;
use crate::state::{ClientState, SubsurfaceIndex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubsurfaceId(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubsurfacePlacement {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug)]
pub(crate) struct Subsurface {
    surface: WlSurface,
    role: WlSubsurface,
    buffer: WlBuffer,
}

delegate_noop!(ClientState: WlSubcompositor);
delegate_noop!(ClientState: WlSubsurface);

impl Client {
    #[must_use]
    pub fn subsurface_under_pointer(&self) -> Option<SubsurfaceId> {
        self.state.seat.pointer_subsurface.map(SubsurfaceId)
    }

    pub fn create_subsurface(
        &mut self,
        parent: ToplevelId,
        placement: SubsurfacePlacement,
    ) -> Result<SubsurfaceId, ClientError> {
        let handle = self.queue.handle();
        let parent = self.surface_of(parent)?;
        let index = SubsurfaceIndex(self.subsurfaces.len());
        let buffer = buffer::create(&self.shm, &handle, placement.width, placement.height)?;
        let surface = self.compositor.create_surface(&handle, index);
        let role = self
            .subcompositor
            .get_subsurface(&surface, parent, &handle, ());
        role.set_position(placement.x, placement.y);
        surface.attach(Some(&buffer), 0, 0);
        surface.commit();
        self.subsurfaces.push(Some(Subsurface {
            surface,
            role,
            buffer,
        }));
        Ok(SubsurfaceId(index.0))
    }

    pub fn destroy_subsurface(&mut self, subsurface: SubsurfaceId) -> Result<(), ClientError> {
        let target = self
            .subsurfaces
            .get_mut(subsurface.0)
            .and_then(Option::take)
            .ok_or(ClientError::DestroyedSubsurface)?;
        target.role.destroy();
        target.surface.destroy();
        target.buffer.destroy();
        self.roundtrip()
    }
}

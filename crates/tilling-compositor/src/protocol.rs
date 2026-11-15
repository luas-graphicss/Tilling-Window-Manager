mod compositor;
mod decoration;
#[cfg(feature = "drm")]
mod dmabuf;
mod fractional_scale;
mod output;
mod seat;
mod xdg_shell;

#[cfg(feature = "drm")]
use smithay::delegate_dmabuf;
use smithay::{
    delegate_compositor, delegate_fractional_scale, delegate_output, delegate_presentation,
    delegate_seat, delegate_shm, delegate_viewporter, delegate_xdg_decoration, delegate_xdg_shell,
};

use crate::runtime::Compositor;
#[cfg(feature = "drm")]
pub(crate) use dmabuf::DmabufImports;

delegate_compositor!(Compositor);
delegate_shm!(Compositor);
delegate_output!(Compositor);
delegate_seat!(Compositor);
delegate_xdg_shell!(Compositor);
delegate_xdg_decoration!(Compositor);
delegate_viewporter!(Compositor);
delegate_fractional_scale!(Compositor);
delegate_presentation!(Compositor);
#[cfg(feature = "drm")]
delegate_dmabuf!(Compositor);

mod compositor;
mod decoration;
mod fractional_scale;
mod output;
mod seat;
mod xdg_shell;

use smithay::{
    delegate_compositor, delegate_fractional_scale, delegate_output, delegate_seat, delegate_shm,
    delegate_viewporter, delegate_xdg_decoration, delegate_xdg_shell,
};

use crate::runtime::Compositor;

delegate_compositor!(Compositor);
delegate_shm!(Compositor);
delegate_output!(Compositor);
delegate_seat!(Compositor);
delegate_xdg_shell!(Compositor);
delegate_xdg_decoration!(Compositor);
delegate_viewporter!(Compositor);
delegate_fractional_scale!(Compositor);

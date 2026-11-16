mod activation;
mod compositor;
mod decoration;
#[cfg(feature = "drm")]
mod dmabuf;
mod fractional_scale;
mod idle;
mod layer_shell;
mod output;
mod pointer_constraints;
mod seat;
mod selection;
mod session_lock;
mod states;
mod text_input;
mod xdg_shell;

#[cfg(feature = "drm")]
use smithay::delegate_dmabuf;
use smithay::{
    delegate_compositor, delegate_cursor_shape, delegate_data_control, delegate_data_device,
    delegate_fractional_scale, delegate_idle_inhibit, delegate_input_method_manager,
    delegate_layer_shell, delegate_output, delegate_pointer_constraints, delegate_presentation,
    delegate_primary_selection, delegate_relative_pointer, delegate_seat, delegate_session_lock,
    delegate_shm, delegate_text_input_manager, delegate_viewporter,
    delegate_virtual_keyboard_manager, delegate_xdg_activation, delegate_xdg_decoration,
    delegate_xdg_shell,
};

use crate::runtime::Compositor;
pub(crate) use compositor::root_of;
#[cfg(feature = "drm")]
pub(crate) use dmabuf::DmabufImports;
pub(crate) use states::DesktopProtocols;

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
delegate_layer_shell!(Compositor);
delegate_data_device!(Compositor);
delegate_primary_selection!(Compositor);
delegate_data_control!(Compositor);
delegate_text_input_manager!(Compositor);
delegate_input_method_manager!(Compositor);
delegate_virtual_keyboard_manager!(Compositor);
delegate_cursor_shape!(Compositor);
delegate_relative_pointer!(Compositor);
delegate_pointer_constraints!(Compositor);
delegate_xdg_activation!(Compositor);
delegate_idle_inhibit!(Compositor);
delegate_session_lock!(Compositor);

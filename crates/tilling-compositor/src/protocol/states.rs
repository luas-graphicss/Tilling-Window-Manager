use smithay::reexports::wayland_protocols::ext::idle_notify::v1::server::ext_idle_notifier_v1::ExtIdleNotifierV1;
use smithay::reexports::wayland_server::DisplayHandle;
use smithay::wayland::cursor_shape::CursorShapeManagerState;
use smithay::wayland::idle_inhibit::IdleInhibitManagerState;
use smithay::wayland::input_method::InputMethodManagerState;
use smithay::wayland::pointer_constraints::PointerConstraintsState;
use smithay::wayland::relative_pointer::RelativePointerManagerState;
use smithay::wayland::selection::data_device::DataDeviceState;
use smithay::wayland::selection::primary_selection::PrimarySelectionState;
use smithay::wayland::selection::wlr_data_control::DataControlState;
use smithay::wayland::session_lock::SessionLockManagerState;
use smithay::wayland::shell::wlr_layer::WlrLayerShellState;
use smithay::wayland::text_input::TextInputManagerState;
use smithay::wayland::virtual_keyboard::VirtualKeyboardManagerState;
use smithay::wayland::xdg_activation::XdgActivationState;

use crate::runtime::Compositor;
use crate::security::{Capability, client_may};

const IDLE_NOTIFIER_VERSION: u32 = 2;

pub(crate) struct DesktopProtocols {
    pub(crate) layer_shell: WlrLayerShellState,
    pub(crate) data_device: DataDeviceState,
    pub(crate) primary_selection: PrimarySelectionState,
    pub(crate) data_control: DataControlState,
    pub(crate) activation: XdgActivationState,
    pub(crate) session_lock: SessionLockManagerState,
}

impl DesktopProtocols {
    pub(crate) fn new(display: &DisplayHandle) -> Self {
        let primary_selection = PrimarySelectionState::new::<Compositor>(display);
        let data_control =
            DataControlState::new::<Compositor, _>(display, Some(&primary_selection), |client| {
                client_may(client, Capability::DataControl)
            });
        TextInputManagerState::new::<Compositor>(display);
        InputMethodManagerState::new::<Compositor, _>(display, |client| {
            client_may(client, Capability::InputMethod)
        });
        VirtualKeyboardManagerState::new::<Compositor, _>(display, |client| {
            client_may(client, Capability::VirtualKeyboard)
        });
        CursorShapeManagerState::new::<Compositor>(display);
        RelativePointerManagerState::new::<Compositor>(display);
        PointerConstraintsState::new::<Compositor>(display);
        IdleInhibitManagerState::new::<Compositor>(display);
        display.create_global::<Compositor, ExtIdleNotifierV1, ()>(IDLE_NOTIFIER_VERSION, ());
        Self {
            layer_shell: WlrLayerShellState::new::<Compositor>(display),
            data_device: DataDeviceState::new::<Compositor>(display),
            primary_selection,
            data_control,
            activation: XdgActivationState::new::<Compositor>(display),
            session_lock: SessionLockManagerState::new::<Compositor, _>(display, |client| {
                client_may(client, Capability::SessionLock)
            }),
        }
    }
}

use smithay::reexports::wayland_protocols::ext::idle_notify::v1::server::ext_idle_notification_v1::{
    self, ExtIdleNotificationV1,
};
use smithay::reexports::wayland_protocols::ext::idle_notify::v1::server::ext_idle_notifier_v1::{
    self, ExtIdleNotifierV1,
};
use smithay::reexports::wayland_server::backend::ClientId;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{
    Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New,
};
use smithay::wayland::idle_inhibit::IdleInhibitHandler;

use crate::idle::InhibitorPolicy;
use crate::runtime::Compositor;

impl GlobalDispatch<ExtIdleNotifierV1, ()> for Compositor {
    fn bind(
        _state: &mut Self,
        _display: &DisplayHandle,
        _client: &Client,
        resource: New<ExtIdleNotifierV1>,
        (): &(),
        data_init: &mut DataInit<'_, Self>,
    ) {
        data_init.init(resource, ());
    }
}

impl Dispatch<ExtIdleNotifierV1, ()> for Compositor {
    fn request(
        state: &mut Self,
        _client: &Client,
        _notifier: &ExtIdleNotifierV1,
        request: ext_idle_notifier_v1::Request,
        (): &(),
        _display: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        match request {
            ext_idle_notifier_v1::Request::GetIdleNotification { id, timeout, .. } => {
                let notification = data_init.init(id, ());
                state.watch_idleness(notification, timeout, InhibitorPolicy::Obeyed);
            }
            ext_idle_notifier_v1::Request::GetInputIdleNotification { id, timeout, .. } => {
                let notification = data_init.init(id, ());
                state.watch_idleness(notification, timeout, InhibitorPolicy::Ignored);
            }
            _ => {}
        }
    }
}

impl Dispatch<ExtIdleNotificationV1, ()> for Compositor {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _notification: &ExtIdleNotificationV1,
        _request: ext_idle_notification_v1::Request,
        (): &(),
        _display: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
    }

    fn destroyed(
        state: &mut Self,
        _client: ClientId,
        notification: &ExtIdleNotificationV1,
        (): &(),
    ) {
        state.idle.forget(notification);
    }
}

impl IdleInhibitHandler for Compositor {
    fn inhibit(&mut self, surface: WlSurface) {
        self.idle.inhibit(surface);
    }

    fn uninhibit(&mut self, surface: WlSurface) {
        self.release_idle_inhibitor(&surface);
    }
}

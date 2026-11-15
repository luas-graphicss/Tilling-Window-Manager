mod clients;
pub(crate) mod instance;
pub(crate) mod session;
mod signals;
mod state;

use std::time::Duration;

use smithay::reexports::calloop::EventLoop;
use smithay::reexports::wayland_server::Display;

use crate::backend::Backend;
use crate::control;
use crate::error::CompositorError;
use crate::input::Launcher;
use crate::options::Options;
use instance::Instance;
use state::Settings;

pub(crate) use clients::ClientState;
pub(crate) use state::Compositor;

pub(crate) struct Runtime {
    pub(crate) display: Display<Compositor>,
    pub(crate) compositor: Compositor,
    pub(crate) backend: Backend,
}

pub fn run(options: Options) -> Result<(), CompositorError> {
    let mut event_loop = EventLoop::try_new().map_err(CompositorError::EventLoop)?;
    signals::watch(&event_loop.handle())?;
    let (mut runtime, instance) = Runtime::launch(&event_loop, options)?;
    tracing::info!(
        wayland_socket = %instance.wayland_socket().display(),
        control_socket = %instance.control_socket().display(),
        "accepting clients"
    );
    runtime.serve(&mut event_loop)
}

impl Runtime {
    pub(crate) fn launch(
        event_loop: &EventLoop<'static, Self>,
        options: Options,
    ) -> Result<(Self, Instance), CompositorError> {
        let instance = Instance::resolve(&options.runtime_dir, &options.instance)?;
        let mut display = Display::new().map_err(CompositorError::Display)?;
        let handle = event_loop.handle();
        let settings = Settings {
            config: options.config,
            config_source: options.config_source,
            transaction_timeout: options.transaction_timeout,
            output_confirmation: options.output_confirmation,
            launcher: Launcher::new(&options.runtime_dir, instance.wayland_socket()),
        };
        let mut compositor = Compositor::new(
            display.handle(),
            handle.clone(),
            event_loop.get_signal(),
            settings,
        )?;
        let backend = Backend::start(options.backend, &options.output, &mut compositor)?;
        clients::watch_display(&handle, &mut display)?;
        clients::listen(&handle, instance.wayland_socket())?;
        control::listen(&handle, instance.control_socket())?;
        let runtime = Self {
            display,
            compositor,
            backend,
        };
        Ok((runtime, instance))
    }

    pub(crate) fn serve(
        &mut self,
        event_loop: &mut EventLoop<'static, Self>,
    ) -> Result<(), CompositorError> {
        event_loop
            .run(None::<Duration>, self, Self::after_dispatch)
            .map_err(CompositorError::Dispatch)
    }

    pub(crate) fn present(&mut self) {
        self.backend.present(&mut self.compositor);
    }

    fn after_dispatch(&mut self) {
        self.compositor.settle();
        self.compositor.popups.cleanup();
        self.backend.follow(&mut self.compositor);
        if self.compositor.take_frame_demand() {
            self.backend.schedule_frame(&mut self.compositor);
        }
        if let Err(error) = self.display.flush_clients() {
            tracing::warn!(%error, "flush wayland clients");
        }
    }
}

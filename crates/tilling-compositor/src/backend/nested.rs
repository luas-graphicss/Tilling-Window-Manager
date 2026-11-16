use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::winit::{self, WinitEvent, WinitGraphicsBackend};
use smithay::input::pointer::{CursorIcon, CursorImageStatus};
use smithay::output::{Mode, Output};
use smithay::reexports::winit::dpi::PhysicalSize;
use smithay::reexports::winit::window::Window;

use crate::error::{CompositorError, cause_chain};
use crate::options::VirtualOutput;
use crate::output::OutputDescription;
use crate::render::{FramebufferOrigin, Painter};
use crate::runtime::Compositor;

const REFRESH_MILLIHERTZ: i32 = 60_000;
const TITLE: &str = "Tilling";

pub(crate) struct NestedBackend {
    output: Output,
    host: WinitGraphicsBackend<GlesRenderer>,
    painter: Painter,
    host_cursor: Option<CursorIcon>,
}

impl NestedBackend {
    pub(crate) fn start(
        output: &VirtualOutput,
        compositor: &mut Compositor,
    ) -> Result<Self, CompositorError> {
        let attributes = Window::default_attributes()
            .with_inner_size(PhysicalSize::new(output.width, output.height))
            .with_title(TITLE);
        let (host, events) = winit::init_from_attributes::<GlesRenderer>(attributes)
            .map_err(|error| CompositorError::HostWindow(cause_chain(&error)))?;
        let description = OutputDescription::virtual_output(
            &output.name,
            host.window_size(),
            REFRESH_MILLIHERTZ,
            host.scale_factor(),
        );
        let output = compositor.attach_output(description)?;
        compositor
            .loop_handle
            .insert_source(events, |event, (), runtime| runtime.host_event(event))
            .map_err(|error| CompositorError::EventSource(error.error))?;
        compositor.demand_frame();
        Ok(Self {
            painter: Painter::new(&output, FramebufferOrigin::BottomLeft),
            output,
            host,
            host_cursor: Some(CursorIcon::Default),
        })
    }

    pub(crate) fn schedule_frame(&self) {
        self.host.window().request_redraw();
    }

    pub(crate) fn handle_event(&mut self, event: WinitEvent, compositor: &mut Compositor) {
        match event {
            WinitEvent::Resized { size, scale_factor } => {
                let mode = Mode {
                    size,
                    refresh: REFRESH_MILLIHERTZ,
                };
                compositor.resize_output(&self.output, mode, scale_factor);
            }
            WinitEvent::Input(event) => compositor.handle_device_event(event, Some(&self.output)),
            WinitEvent::Focus(false) => compositor.release_held_keys(),
            WinitEvent::Focus(true) => {}
            WinitEvent::Redraw => self.present(compositor),
            WinitEvent::CloseRequested => compositor.stop(),
        }
    }

    pub(crate) fn present(&mut self, compositor: &mut Compositor) {
        self.show_host_cursor(compositor);
        let buffer_age = self.host.buffer_age().unwrap_or(0);
        let painted = match self.host.bind() {
            Ok((renderer, mut framebuffer)) => {
                self.painter
                    .paint(compositor, renderer, &mut framebuffer, buffer_age)
            }
            Err(error) => {
                tracing::warn!(%error, "bind the host window");
                return;
            }
        };
        match painted {
            Ok(damage) => {
                if let Some(damage) = &damage
                    && let Err(error) = self.host.submit(Some(damage))
                {
                    tracing::warn!(%error, "submit frame to the host window");
                }
                compositor.frames.record(damage.as_deref());
            }
            Err(error) => tracing::warn!(%error, "paint nested frame"),
        }
        compositor.send_frame_callbacks(&self.output);
    }

    fn show_host_cursor(&mut self, compositor: &Compositor) {
        let wanted = match &compositor.cursor {
            CursorImageStatus::Named(icon) => Some(*icon),
            CursorImageStatus::Surface(_) | CursorImageStatus::Hidden => None,
        };
        if wanted == self.host_cursor {
            return;
        }
        let window = self.host.window();
        if let Some(icon) = wanted {
            window.set_cursor(icon);
        }
        window.set_cursor_visible(wanted.is_some());
        self.host_cursor = wanted;
    }
}

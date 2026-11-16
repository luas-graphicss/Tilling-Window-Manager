use std::time::{Duration, Instant};

use smithay::backend::renderer::test::{DummyFramebuffer, DummyRenderer};
use smithay::output::Output;
use smithay::reexports::calloop::LoopHandle;
use smithay::reexports::calloop::timer::{TimeoutAction, Timer};
use smithay::utils::{Physical, Size};

use crate::error::CompositorError;
use crate::options::VirtualOutput;
use crate::output::OutputDescription;
use crate::render::{FramebufferOrigin, Painter};
use crate::runtime::{Compositor, Runtime};

const REFRESH_MILLIHERTZ: i32 = 60_000;
const FRAME_INTERVAL: Duration = Duration::from_micros(16_667);
const RETAINED_BUFFER_AGE: usize = 1;
const UNSCALED: f64 = 1.0;

struct Screen {
    output: Output,
    painter: Painter,
}

pub(crate) struct HeadlessBackend {
    screens: Vec<Screen>,
    renderer: DummyRenderer,
    last_frame: Option<Instant>,
    frame_scheduled: bool,
}

impl HeadlessBackend {
    pub(crate) fn start(
        output: &VirtualOutput,
        compositor: &mut Compositor,
    ) -> Result<Self, CompositorError> {
        let mut backend = Self {
            screens: Vec::new(),
            renderer: DummyRenderer,
            last_frame: None,
            frame_scheduled: false,
        };
        backend.plug(output, compositor)?;
        Ok(backend)
    }

    pub(crate) fn plug(
        &mut self,
        output: &VirtualOutput,
        compositor: &mut Compositor,
    ) -> Result<(), CompositorError> {
        if self.position_of(&output.name).is_some() {
            return Err(CompositorError::OutputInUse(output.name.clone()));
        }
        let description = OutputDescription::virtual_output(
            &output.name,
            physical_size(output)?,
            REFRESH_MILLIHERTZ,
            UNSCALED,
        );
        let output = compositor.attach_output(description)?;
        self.screens.push(Screen {
            painter: Painter::new(&output, FramebufferOrigin::TopLeft),
            output,
        });
        Ok(())
    }

    pub(crate) fn unplug(
        &mut self,
        name: &str,
        compositor: &mut Compositor,
    ) -> Result<(), CompositorError> {
        let position = self
            .position_of(name)
            .ok_or_else(|| CompositorError::UnknownOutput(name.to_owned()))?;
        let screen = self.screens.remove(position);
        compositor.detach_output(&screen.output);
        Ok(())
    }

    pub(crate) fn schedule_frame(&mut self, handle: &LoopHandle<'static, Runtime>) {
        if self.frame_scheduled {
            return;
        }
        let wait = self.last_frame.map_or(Duration::ZERO, |last| {
            FRAME_INTERVAL.saturating_sub(last.elapsed())
        });
        let timer = Timer::from_duration(wait);
        let scheduled = handle.insert_source(timer, |_, (), runtime| {
            runtime.present();
            TimeoutAction::Drop
        });
        match scheduled {
            Ok(_) => self.frame_scheduled = true,
            Err(error) => tracing::warn!(error = %error.error, "schedule headless frame"),
        }
    }

    pub(crate) fn present(&mut self, compositor: &mut Compositor) {
        self.frame_scheduled = false;
        self.last_frame = Some(Instant::now());
        for screen in &mut self.screens {
            let painted = screen.painter.paint(
                compositor,
                &mut self.renderer,
                &mut DummyFramebuffer,
                RETAINED_BUFFER_AGE,
            );
            match painted {
                Ok(damage) => compositor.frames.record(damage.as_deref()),
                Err(error) => tracing::warn!(%error, "paint headless frame"),
            }
            compositor.send_frame_callbacks(&screen.output);
        }
    }

    fn position_of(&self, name: &str) -> Option<usize> {
        self.screens
            .iter()
            .position(|screen| screen.output.name() == name)
    }
}

fn physical_size(output: &VirtualOutput) -> Result<Size<i32, Physical>, CompositorError> {
    let unusable = || CompositorError::OutputSize {
        width: output.width,
        height: output.height,
    };
    let width = i32::try_from(output.width).map_err(|_| unusable())?;
    let height = i32::try_from(output.height).map_err(|_| unusable())?;
    if width == 0 || height == 0 {
        return Err(unusable());
    }
    Ok(Size::from((width, height)))
}

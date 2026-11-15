use std::time::{Duration, Instant};

use smithay::output::Output;
use smithay::reexports::calloop::LoopHandle;
use smithay::reexports::calloop::timer::{TimeoutAction, Timer};
use smithay::utils::{Physical, Size};

use crate::error::CompositorError;
use crate::options::VirtualOutput;
use crate::runtime::{Compositor, Runtime};

const REFRESH_MILLIHERTZ: i32 = 60_000;
const FRAME_INTERVAL: Duration = Duration::from_micros(16_667);

pub(crate) struct HeadlessBackend {
    output: Output,
    last_frame: Option<Instant>,
    frame_scheduled: bool,
}

impl HeadlessBackend {
    pub(crate) fn start(
        output: &VirtualOutput,
        compositor: &mut Compositor,
    ) -> Result<Self, CompositorError> {
        let size = physical_size(output)?;
        let output = compositor.attach_output(&output.name, size, REFRESH_MILLIHERTZ)?;
        Ok(Self {
            output,
            last_frame: None,
            frame_scheduled: false,
        })
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
        compositor.send_frame_callbacks(&self.output);
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

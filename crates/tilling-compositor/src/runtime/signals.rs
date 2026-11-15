use smithay::reexports::calloop::LoopHandle;
use smithay::reexports::calloop::signals::{Signal, Signals};

use super::Runtime;
use crate::error::CompositorError;

pub(crate) fn watch(handle: &LoopHandle<'static, Runtime>) -> Result<(), CompositorError> {
    let signals =
        Signals::new(&[Signal::SIGINT, Signal::SIGTERM]).map_err(CompositorError::Signals)?;
    handle
        .insert_source(signals, |event, _, runtime| {
            tracing::info!(signal = ?event.signal(), "stop on signal");
            runtime.compositor.stop();
        })
        .map_err(|error| CompositorError::EventSource(error.error))?;
    Ok(())
}

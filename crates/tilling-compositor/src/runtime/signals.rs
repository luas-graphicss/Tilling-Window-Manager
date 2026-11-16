use std::ffi::c_int;
use std::io;
use std::os::unix::net::UnixStream;

use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::low_level::pipe;
use smithay::reexports::calloop::generic::Generic;
use smithay::reexports::calloop::{Interest, LoopHandle, Mode, PostAction};

use super::Runtime;
use crate::error::CompositorError;

const STOP_SIGNALS: [(c_int, &str); 2] = [(SIGINT, "SIGINT"), (SIGTERM, "SIGTERM")];

pub(crate) fn watch(handle: &LoopHandle<'static, Runtime>) -> Result<(), CompositorError> {
    for (signal, name) in STOP_SIGNALS {
        let arrivals = arrivals_of(signal).map_err(CompositorError::Signals)?;
        let source = Generic::new(arrivals, Interest::READ, Mode::Level);
        handle
            .insert_source(source, move |_, _, runtime| {
                tracing::info!(signal = %name, "stop on signal");
                runtime.compositor.stop();
                Ok(PostAction::Disable)
            })
            .map_err(|error| CompositorError::EventSource(error.error))?;
    }
    Ok(())
}

fn arrivals_of(signal: c_int) -> io::Result<UnixStream> {
    let (arrivals, notifier) = UnixStream::pair()?;
    pipe::register(signal, notifier)?;
    Ok(arrivals)
}

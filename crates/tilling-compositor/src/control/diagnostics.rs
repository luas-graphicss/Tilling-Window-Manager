use tilling_ipc::DiagnosticsInfo;

use crate::runtime::{Compositor, ProcessUsage};

impl Compositor {
    pub(super) fn diagnostics(&self) -> DiagnosticsInfo {
        let usage = ProcessUsage::measure();
        DiagnosticsInfo {
            revision: self.desktop.revision(),
            outputs: count(self.desktop.outputs().count()),
            workspaces: count(self.desktop.workspaces().count()),
            windows: count(self.desktop.windows().count()),
            input_devices: count(self.input_devices),
            frames_presented: self.frames.presented,
            frames_damaged: self.frames.damaged,
            resident_kib: usage.resident_kib,
            open_descriptors: usage.open_descriptors,
            cpu_time_ms: usage.cpu_time_ms,
        }
    }
}

fn count(length: usize) -> u32 {
    u32::try_from(length).unwrap_or(u32::MAX)
}

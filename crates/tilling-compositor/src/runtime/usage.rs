use std::fs;

use rustix::time::{ClockId, clock_gettime};

const STATUS_PATH: &str = "/proc/self/status";
const DESCRIPTORS_PATH: &str = "/proc/self/fd";
const RESIDENT_FIELD: &str = "VmRSS:";
const LISTING_DESCRIPTORS: usize = 1;
const MILLISECONDS_PER_SECOND: u64 = 1_000;
const NANOSECONDS_PER_MILLISECOND: u64 = 1_000_000;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ProcessUsage {
    pub(crate) resident_kib: u64,
    pub(crate) open_descriptors: u32,
    pub(crate) cpu_time_ms: u64,
}

impl ProcessUsage {
    pub(crate) fn measure() -> Self {
        let resident_kib = fs::read_to_string(STATUS_PATH)
            .ok()
            .and_then(|status| resident_kib(&status))
            .unwrap_or(0);
        let open_descriptors = fs::read_dir(DESCRIPTORS_PATH).map_or(0, |entries| {
            let open = entries.count().saturating_sub(LISTING_DESCRIPTORS);
            u32::try_from(open).unwrap_or(u32::MAX)
        });
        let spent = clock_gettime(ClockId::ProcessCPUTime);
        let seconds = u64::try_from(spent.tv_sec).unwrap_or(0);
        let nanoseconds = u64::try_from(spent.tv_nsec).unwrap_or(0);
        Self {
            resident_kib,
            open_descriptors,
            cpu_time_ms: seconds
                .saturating_mul(MILLISECONDS_PER_SECOND)
                .saturating_add(nanoseconds / NANOSECONDS_PER_MILLISECOND),
        }
    }
}

fn resident_kib(status: &str) -> Option<u64> {
    status
        .lines()
        .find_map(|line| line.strip_prefix(RESIDENT_FIELD))?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resident_memory_is_read_from_the_status_table() {
        let status = "Name:\ttilling\nVmPeak:\t  912000 kB\nVmRSS:\t   48212 kB\nThreads:\t3\n";

        assert_eq!(resident_kib(status), Some(48_212));
        assert_eq!(resident_kib("Name:\ttilling\n"), None);
        assert_eq!(resident_kib("VmRSS:\tunknown kB\n"), None);
    }

    #[test]
    fn running_process_reports_memory_and_descriptors() {
        let usage = ProcessUsage::measure();

        assert!(usage.resident_kib > 0);
        assert!(usage.open_descriptors > 0);
    }
}

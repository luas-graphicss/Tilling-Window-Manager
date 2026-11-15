use std::fs;
use std::path::Path;

const KERNEL_RELEASE: &str = "/proc/sys/kernel/osrelease";
const GRAPHICS_CLASS: &str = "/sys/class/drm";
const DEVICE_PREFIX: &str = "card";
const CONNECTOR_SEPARATOR: char = '-';
const UNKNOWN: &str = "unknown";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inventory {
    kernel: String,
    graphics: Vec<String>,
}

impl Inventory {
    pub fn read() -> Self {
        Self {
            kernel: first_line(Path::new(KERNEL_RELEASE)),
            graphics: graphics(Path::new(GRAPHICS_CLASS)),
        }
    }

    pub fn has_graphics(&self) -> bool {
        !self.graphics.is_empty()
    }

    pub fn lines(&self) -> Vec<String> {
        let mut lines = vec![format!("kernel: {}", self.kernel)];
        if self.graphics.is_empty() {
            lines.push("graphics devices: none found".to_owned());
        }
        lines.extend(self.graphics.iter().cloned());
        lines
    }
}

fn graphics(class: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(class)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.starts_with(DEVICE_PREFIX))
        .collect();
    names.sort();
    names.iter().map(|name| entry(class, name)).collect()
}

fn entry(class: &Path, name: &str) -> String {
    let directory = class.join(name);
    if name.contains(CONNECTOR_SEPARATOR) {
        let status = first_line(&directory.join("status"));
        let mode = first_line(&directory.join("modes"));
        return format!("connector {name}: {status}, first mode {mode}");
    }
    let driver = fs::read_link(directory.join("device/driver"))
        .ok()
        .and_then(|target| Some(target.file_name()?.to_string_lossy().into_owned()))
        .unwrap_or_else(|| UNKNOWN.to_owned());
    format!("graphics device {name}: driver {driver}")
}

fn first_line(path: &Path) -> String {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| text.lines().next().map(str::to_owned))
        .filter(|line| !line.is_empty())
        .unwrap_or_else(|| UNKNOWN.to_owned())
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::symlink;

    use super::*;
    use crate::sandbox::Sandbox;

    #[test]
    fn devices_and_connectors_are_listed_in_a_stable_order() {
        let sandbox = Sandbox::create(None, "inventory").unwrap();
        let class = sandbox.path();
        for directory in [
            "card0/device",
            "card0-eDP-1",
            "card0-HDMI-A-1",
            "renderD128",
        ] {
            fs::create_dir_all(class.join(directory)).unwrap();
        }
        symlink(
            "../../bus/pci/drivers/i915",
            class.join("card0/device/driver"),
        )
        .unwrap();
        fs::write(class.join("card0-eDP-1/status"), "connected\n").unwrap();
        fs::write(class.join("card0-eDP-1/modes"), "1920x1080\n1280x720\n").unwrap();
        fs::write(class.join("card0-HDMI-A-1/status"), "disconnected\n").unwrap();
        fs::write(class.join("card0-HDMI-A-1/modes"), "").unwrap();

        assert_eq!(
            graphics(class),
            [
                "graphics device card0: driver i915",
                "connector card0-HDMI-A-1: disconnected, first mode unknown",
                "connector card0-eDP-1: connected, first mode 1920x1080",
            ]
        );
    }

    #[test]
    fn machine_without_graphics_devices_says_so() {
        let sandbox = Sandbox::create(None, "no-graphics").unwrap();
        let inventory = Inventory {
            kernel: "6.12.9".to_owned(),
            graphics: graphics(&sandbox.path().join("missing")),
        };

        assert!(!inventory.has_graphics());
        assert_eq!(
            inventory.lines(),
            ["kernel: 6.12.9", "graphics devices: none found"]
        );
    }
}

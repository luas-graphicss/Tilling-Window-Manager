use smithay::output::{Mode, Subpixel};
use smithay::utils::{Physical, Raw, Size};

const VIRTUAL_MAKE: &str = "Tilling";
const VIRTUAL_MODEL: &str = "Virtual output";

#[derive(Debug, Clone)]
pub(crate) struct OutputDescription {
    pub(crate) name: String,
    pub(crate) monitor_key: Option<String>,
    pub(crate) make: String,
    pub(crate) model: String,
    pub(crate) physical_size: Size<i32, Raw>,
    pub(crate) subpixel: Subpixel,
    pub(crate) mode: Mode,
    pub(crate) suggested_scale: f64,
}

impl OutputDescription {
    pub(crate) fn virtual_output(
        name: &str,
        size: Size<i32, Physical>,
        refresh_millihertz: i32,
        suggested_scale: f64,
    ) -> Self {
        Self {
            name: name.to_owned(),
            monitor_key: None,
            make: VIRTUAL_MAKE.to_owned(),
            model: VIRTUAL_MODEL.to_owned(),
            physical_size: Size::from((0, 0)),
            subpixel: Subpixel::Unknown,
            mode: Mode {
                size,
                refresh: refresh_millihertz,
            },
            suggested_scale,
        }
    }
}

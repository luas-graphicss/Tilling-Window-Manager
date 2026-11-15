use tilling_core::geometry::Scale;

use crate::diagnostics::Report;
use crate::limits::{MAX_OUTPUT_EDGE, MAX_OUTPUTS, MAX_REFRESH_HZ, MIN_REFRESH_HZ};
use crate::names::check_name;
use crate::schema::{RawMode, RawOutput, RawTransform};

const MILLIHERTZ_PER_HERTZ: f64 = 1_000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModeConfig {
    pub width: u32,
    pub height: u32,
    pub refresh_millihertz: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputTransform {
    Normal,
    Rotated90,
    Rotated180,
    Rotated270,
    Flipped,
    Flipped90,
    Flipped180,
    Flipped270,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputConfig {
    pub name: String,
    pub monitor: Option<String>,
    pub scale: Option<Scale>,
    pub position: Option<Position>,
    pub mode: Option<ModeConfig>,
    pub transform: Option<OutputTransform>,
}

impl From<RawTransform> for OutputTransform {
    fn from(raw: RawTransform) -> Self {
        match raw {
            RawTransform::Normal => Self::Normal,
            RawTransform::Rotated90 => Self::Rotated90,
            RawTransform::Rotated180 => Self::Rotated180,
            RawTransform::Rotated270 => Self::Rotated270,
            RawTransform::Flipped => Self::Flipped,
            RawTransform::Flipped90 => Self::Flipped90,
            RawTransform::Flipped180 => Self::Flipped180,
            RawTransform::Flipped270 => Self::Flipped270,
        }
    }
}

pub(crate) fn validate(raw: Vec<RawOutput>, report: &mut Report) -> Vec<OutputConfig> {
    if raw.len() > MAX_OUTPUTS {
        report.reject(
            "outputs",
            format!("must not list more than {MAX_OUTPUTS} entries"),
        );
    }
    let mut outputs: Vec<OutputConfig> = Vec::new();
    for (index, entry) in raw.into_iter().take(MAX_OUTPUTS).enumerate() {
        let name_field = format!("outputs[{index}].name");
        if outputs.iter().any(|earlier| earlier.name == entry.name) {
            report.reject(
                name_field.as_str(),
                format!("`{}` is configured more than once", entry.name),
            );
        }
        report.accept(name_field, check_name(&entry.name));
        let monitor = entry
            .monitor
            .and_then(|key| validate_monitor(key, index, &outputs, report));
        let scale = entry.scale.and_then(|factor| {
            report.accept(
                format!("outputs[{index}].scale"),
                Scale::from_factor(factor),
            )
        });
        let position = entry.position.map(|position| Position {
            x: position.x,
            y: position.y,
        });
        let mode = entry
            .mode
            .and_then(|mode| validate_mode(mode, index, report));
        outputs.push(OutputConfig {
            name: entry.name,
            monitor,
            scale,
            position,
            mode,
            transform: entry.transform.map(OutputTransform::from),
        });
    }
    outputs
}

fn validate_monitor(
    key: String,
    index: usize,
    earlier: &[OutputConfig],
    report: &mut Report,
) -> Option<String> {
    let field = format!("outputs[{index}].monitor");
    let repeated = earlier
        .iter()
        .any(|output| output.monitor.as_deref() == Some(key.as_str()));
    if repeated {
        report.reject(
            field.as_str(),
            format!("`{key}` is configured more than once"),
        );
    }
    report.accept(field, check_name(&key))?;
    Some(key)
}

fn validate_mode(raw: RawMode, index: usize, report: &mut Report) -> Option<ModeConfig> {
    let edges = 1..=MAX_OUTPUT_EDGE;
    if !edges.contains(&raw.width) || !edges.contains(&raw.height) {
        report.reject(
            format!("outputs[{index}].mode"),
            format!("width and height must be between 1 and {MAX_OUTPUT_EDGE}"),
        );
        return None;
    }
    let refresh_millihertz = match raw.refresh {
        None => None,
        Some(hertz) => {
            let Some(rate) = millihertz(hertz) else {
                report.reject(
                    format!("outputs[{index}].mode.refresh"),
                    format!("must be a rate between {MIN_REFRESH_HZ} and {MAX_REFRESH_HZ} Hz"),
                );
                return None;
            };
            Some(rate)
        }
    };
    Some(ModeConfig {
        width: raw.width,
        height: raw.height,
        refresh_millihertz,
    })
}

fn millihertz(hertz: f64) -> Option<u32> {
    if !hertz.is_finite() || !(MIN_REFRESH_HZ..=MAX_REFRESH_HZ).contains(&hertz) {
        return None;
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the rate was range-checked, so the product lies between 1000 and 1000000"
    )]
    let rounded = (hertz * MILLIHERTZ_PER_HERTZ).round() as u32;
    Some(rounded)
}

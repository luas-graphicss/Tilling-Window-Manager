use tilling_core::geometry::Scale;

use crate::diagnostics::Report;
use crate::limits::MAX_OUTPUTS;
use crate::names::check_name;
use crate::schema::RawOutput;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputConfig {
    pub name: String,
    pub scale: Option<Scale>,
    pub position: Option<Position>,
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
        outputs.push(OutputConfig {
            name: entry.name,
            scale,
            position,
        });
    }
    outputs
}

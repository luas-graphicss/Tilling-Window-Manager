mod applications;
mod bindings;
mod diagnostics;
mod input;
mod layout;
mod limits;
mod names;
mod outputs;
mod rules;
mod schema;
mod source;
mod validation;

pub use applications::{Applications, CommandLine, CommandLineError};
pub use bindings::{Action, Binding, ChordError, DEFAULT_MODE, KeyChord, Modifiers};
pub use diagnostics::{ConfigError, Diagnostic};
pub use input::{InputConfig, KeyboardConfig, PointerConfig, TouchpadConfig};
pub use layout::LayoutConfig;
pub use limits::MAX_CONFIG_BYTES;
pub use outputs::{ModeConfig, OutputConfig, OutputTransform, Position};
pub use rules::{RuleMode, RuleOutcome, WindowRules};
pub use source::{ConfigSource, Environment};
pub use validation::ValidatedConfig;

const BUILT_IN: &str = include_str!("../../../config/default.toml");

pub fn parse(text: &str) -> Result<ValidatedConfig, ConfigError> {
    if text.len() > MAX_CONFIG_BYTES {
        return Err(ConfigError::TooLarge {
            limit: MAX_CONFIG_BYTES,
        });
    }
    let defaults =
        schema::decode(BUILT_IN).map_err(|error| ConfigError::InvalidDefaults(Box::new(error)))?;
    let sections = schema::decode(text)?
        .complete(defaults)
        .ok_or(ConfigError::IncompleteDefaults)?;
    validation::validate(sections)
}

pub fn load(source: &ConfigSource) -> Result<ValidatedConfig, ConfigError> {
    match source {
        ConfigSource::BuiltIn => parse(BUILT_IN),
        ConfigSource::File(path) => parse(&source::read(path)?),
    }
}

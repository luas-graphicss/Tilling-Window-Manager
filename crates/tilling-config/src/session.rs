use std::collections::BTreeSet;

use crate::applications::CommandLine;
use crate::diagnostics::Report;
use crate::limits::MAX_HELPERS;
use crate::names::is_identifier;
use crate::schema::{RawCapability, RawHelper, RawRestart, RawSession};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HelperCapability {
    DataControl,
    InputMethod,
    VirtualKeyboard,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RestartPolicy {
    #[default]
    Never,
    OnFailure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Helper {
    pub name: String,
    pub command: CommandLine,
    pub capabilities: BTreeSet<HelperCapability>,
    pub restart: RestartPolicy,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionConfig {
    pub locker: Option<CommandLine>,
    pub helpers: Vec<Helper>,
}

pub(crate) fn validate(
    session: RawSession,
    helpers: Vec<RawHelper>,
    report: &mut Report,
) -> SessionConfig {
    let locker = session
        .locker
        .and_then(|parts| report.accept("session.locker", CommandLine::from_parts(parts)));
    if helpers.len() > MAX_HELPERS {
        report.reject(
            "helpers",
            format!("must not list more than {MAX_HELPERS} entries"),
        );
    }
    let mut names = BTreeSet::new();
    let mut accepted = Vec::new();
    for (index, raw) in helpers.into_iter().take(MAX_HELPERS).enumerate() {
        let name_field = format!("helpers[{index}].name");
        if !is_identifier(&raw.name) {
            report.reject(
                name_field.as_str(),
                "must use lowercase letters, digits, `-` or `_`",
            );
        }
        if !names.insert(raw.name.clone()) {
            report.reject(
                name_field,
                format!("`{}` is listed more than once", raw.name),
            );
        }
        let command = report.accept(
            format!("helpers[{index}].command"),
            CommandLine::from_parts(raw.command),
        );
        if let Some(command) = command {
            accepted.push(Helper {
                name: raw.name,
                command,
                capabilities: raw.capabilities.into_iter().map(Into::into).collect(),
                restart: raw.restart.into(),
            });
        }
    }
    SessionConfig {
        locker,
        helpers: accepted,
    }
}

impl From<RawCapability> for HelperCapability {
    fn from(raw: RawCapability) -> Self {
        match raw {
            RawCapability::DataControl => Self::DataControl,
            RawCapability::InputMethod => Self::InputMethod,
            RawCapability::VirtualKeyboard => Self::VirtualKeyboard,
        }
    }
}

impl From<RawRestart> for RestartPolicy {
    fn from(raw: RawRestart) -> Self {
        match raw {
            RawRestart::Never => Self::Never,
            RawRestart::OnFailure => Self::OnFailure,
        }
    }
}

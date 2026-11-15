use crate::diagnostics::Report;
use crate::limits::{MAX_ARGUMENT_BYTES, MAX_COMMAND_ARGUMENTS};
use crate::schema::RawApplications;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandLine {
    program: String,
    arguments: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CommandLineError {
    #[error("must name a program")]
    MissingProgram,
    #[error("must not pass more than {MAX_COMMAND_ARGUMENTS} arguments")]
    TooManyArguments,
    #[error("arguments must not exceed {MAX_ARGUMENT_BYTES} bytes")]
    ArgumentTooLong,
    #[error("arguments must not contain NUL bytes")]
    NulByte,
}

impl CommandLine {
    pub(crate) fn from_parts(parts: Vec<String>) -> Result<Self, CommandLineError> {
        if parts.len() > MAX_COMMAND_ARGUMENTS + 1 {
            return Err(CommandLineError::TooManyArguments);
        }
        if parts.iter().any(|part| part.len() > MAX_ARGUMENT_BYTES) {
            return Err(CommandLineError::ArgumentTooLong);
        }
        if parts.iter().any(|part| part.contains('\0')) {
            return Err(CommandLineError::NulByte);
        }
        let mut parts = parts.into_iter();
        let program = parts
            .next()
            .filter(|program| !program.is_empty())
            .ok_or(CommandLineError::MissingProgram)?;
        Ok(Self {
            program,
            arguments: parts.collect(),
        })
    }

    #[must_use]
    pub fn program(&self) -> &str {
        &self.program
    }

    #[must_use]
    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applications {
    pub terminal: CommandLine,
}

pub(crate) fn validate(raw: RawApplications, report: &mut Report) -> Option<Applications> {
    let terminal = report.accept(
        "applications.terminal",
        CommandLine::from_parts(raw.terminal),
    )?;
    Some(Applications { terminal })
}

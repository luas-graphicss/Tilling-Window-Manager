use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub field: String,
    pub message: String,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.field, self.message)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("read {}", .path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("configuration exceeds the {limit} byte limit")]
    TooLarge { limit: usize },
    #[error("line {line}, column {column}: {message}")]
    Syntax {
        line: usize,
        column: usize,
        message: String,
    },
    #[error("schema_version {found} is not supported, this build reads version {supported}")]
    UnsupportedSchemaVersion { found: u32, supported: u32 },
    #[error("{}", summarize(.0))]
    Invalid(Vec<Diagnostic>),
    #[error("built-in defaults are invalid")]
    InvalidDefaults(#[source] Box<ConfigError>),
    #[error("built-in defaults do not define every section")]
    IncompleteDefaults,
}

fn summarize(diagnostics: &[Diagnostic]) -> String {
    let lines: Vec<String> = diagnostics.iter().map(Diagnostic::to_string).collect();
    lines.join("; ")
}

#[derive(Debug, Default)]
pub(crate) struct Report {
    diagnostics: Vec<Diagnostic>,
}

impl Report {
    pub(crate) fn reject(&mut self, field: impl Into<String>, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic {
            field: field.into(),
            message: message.into(),
        });
    }

    pub(crate) fn accept<T, E: fmt::Display>(
        &mut self,
        field: impl Into<String>,
        outcome: Result<T, E>,
    ) -> Option<T> {
        match outcome {
            Ok(value) => Some(value),
            Err(error) => {
                self.reject(field, error.to_string());
                None
            }
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.diagnostics.len()
    }

    pub(crate) fn into_error(self) -> ConfigError {
        ConfigError::Invalid(self.diagnostics)
    }

    pub(crate) fn finish<T>(self, value: T) -> Result<T, ConfigError> {
        if self.diagnostics.is_empty() {
            Ok(value)
        } else {
            Err(self.into_error())
        }
    }
}

pub(crate) fn syntax_error(text: &str, error: &toml::de::Error) -> ConfigError {
    let offset = error.span().map_or(0, |span| span.start);
    let preceding = text.get(..offset).unwrap_or(text);
    let line = preceding.matches('\n').count() + 1;
    let column = preceding
        .rsplit('\n')
        .next()
        .map_or(0, |tail| tail.chars().count())
        + 1;
    ConfigError::Syntax {
        line,
        column,
        message: error.message().to_owned(),
    }
}

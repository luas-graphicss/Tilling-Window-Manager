use crate::diagnostics::Report;
use crate::limits::{MAX_KEY_NAME_BYTES, MAX_NAME_BYTES, MAX_WORKSPACES};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum NameError {
    #[error("must not be empty")]
    Empty,
    #[error("must not exceed {MAX_NAME_BYTES} bytes")]
    TooLong,
    #[error("must not contain control characters")]
    ControlCharacter,
}

pub(crate) fn check_name(name: &str) -> Result<(), NameError> {
    if name.is_empty() {
        return Err(NameError::Empty);
    }
    if name.len() > MAX_NAME_BYTES {
        return Err(NameError::TooLong);
    }
    if name.chars().any(char::is_control) {
        return Err(NameError::ControlCharacter);
    }
    Ok(())
}

pub(crate) fn is_identifier(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_KEY_NAME_BYTES
        && name.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
}

pub(crate) fn validate_workspaces(names: Vec<String>, report: &mut Report) -> Vec<String> {
    if names.is_empty() || names.len() > MAX_WORKSPACES {
        report.reject(
            "workspaces",
            format!("must list between 1 and {MAX_WORKSPACES} names"),
        );
    }
    for (index, name) in names.iter().take(MAX_WORKSPACES).enumerate() {
        let field = format!("workspaces[{index}]");
        if names.iter().take(index).any(|earlier| earlier == name) {
            report.reject(field.as_str(), format!("`{name}` is listed more than once"));
        }
        report.accept(field, check_name(name));
    }
    names
}

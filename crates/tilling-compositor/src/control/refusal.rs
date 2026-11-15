use std::error::Error;

use tilling_core::TransitionError;
use tilling_ipc::Outcome;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Refusal {
    NotFound(String),
    Invalid(String),
    NotPermitted(String),
}

impl Refusal {
    pub(super) fn invalid_because(error: &dyn Error) -> Self {
        let mut message = error.to_string();
        let mut cause = error.source();
        while let Some(source) = cause {
            message.push_str(": ");
            message.push_str(&source.to_string());
            cause = source.source();
        }
        Self::Invalid(message)
    }
}

impl From<Refusal> for Outcome {
    fn from(refusal: Refusal) -> Self {
        match refusal {
            Refusal::NotFound(message) => Self::NotFound { message },
            Refusal::Invalid(message) => Self::ValidationError { message },
            Refusal::NotPermitted(message) => Self::NotPermitted { message },
        }
    }
}

impl From<TransitionError> for Refusal {
    fn from(error: TransitionError) -> Self {
        let message = error.to_string();
        match error {
            TransitionError::UnknownWindow(_)
            | TransitionError::UnknownWorkspace(_)
            | TransitionError::UnknownOutput(_) => Self::NotFound(message),
            TransitionError::EmptyWorkspaceName
            | TransitionError::DuplicateWorkspaceName(_)
            | TransitionError::EmptyOutputArea
            | TransitionError::FullscreenLocked(_)
            | TransitionError::RevisionExhausted
            | TransitionError::Layout(_)
            | TransitionError::Capacity(_)
            | TransitionError::Invariant(_) => Self::Invalid(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::*;

    #[derive(Debug, thiserror::Error)]
    #[error("load configuration")]
    struct Outer(#[source] io::Error);

    #[test]
    fn invalid_refusal_keeps_the_whole_cause_chain() {
        let error = Outer(io::Error::other("file vanished"));

        let refusal = Refusal::invalid_because(&error);

        assert_eq!(
            refusal,
            Refusal::Invalid("load configuration: file vanished".to_owned())
        );
    }

    #[test]
    fn domain_rejections_are_reported_as_invalid_requests() {
        let rejected = Outcome::from(Refusal::from(TransitionError::EmptyWorkspaceName));
        let missing = Outcome::from(Refusal::NotFound("window 4v0 does not exist".to_owned()));

        assert!(matches!(rejected, Outcome::ValidationError { .. }));
        assert!(matches!(missing, Outcome::NotFound { .. }));
    }
}

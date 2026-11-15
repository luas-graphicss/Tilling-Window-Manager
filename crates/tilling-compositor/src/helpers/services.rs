use std::io;
use std::process::{Command, Stdio};
use std::time::Duration;

use super::environment::SessionEnvironment;
use super::process;

const SERVICE_MANAGER: &str = "systemctl";
const USER_SCOPE: &str = "--user";
const ANSWER_LIMIT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionServices {
    pub manager: Vec<String>,
    pub target: String,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ServiceError {
    #[error("no service manager is named")]
    NoManager,
    #[error("run `{0}`")]
    Run(String, #[source] io::Error),
    #[error("`{0}` did not succeed")]
    Refused(String),
}

impl SessionServices {
    #[must_use]
    pub fn systemd(target: String) -> Self {
        Self {
            manager: vec![SERVICE_MANAGER.to_owned()],
            target,
        }
    }

    pub(crate) fn announce(&self, environment: &SessionEnvironment) -> Result<(), ServiceError> {
        self.ask(
            environment,
            "import-environment",
            &SessionEnvironment::SHARED,
        )?;
        self.ask(environment, "start", &["--no-block", &self.target])
    }

    pub(crate) fn withdraw(&self, environment: &SessionEnvironment) -> Result<(), ServiceError> {
        self.ask(environment, "stop", &["--no-block", &self.target])?;
        self.ask(
            environment,
            "unset-environment",
            &SessionEnvironment::SHARED,
        )
    }

    fn ask(
        &self,
        environment: &SessionEnvironment,
        verb: &str,
        arguments: &[&str],
    ) -> Result<(), ServiceError> {
        let (program, leading) = self.manager.split_first().ok_or(ServiceError::NoManager)?;
        let described = format!("{program} {USER_SCOPE} {verb}");
        let mut command = Command::new(program);
        command
            .args(leading)
            .arg(USER_SCOPE)
            .arg(verb)
            .args(arguments)
            .stdin(Stdio::null());
        let finished = process::spawn_command(command, environment, None)
            .and_then(|mut child| process::finish_within(&mut child, ANSWER_LIMIT));
        match finished {
            Ok(true) => Ok(()),
            Ok(false) => Err(ServiceError::Refused(described)),
            Err(source) => Err(ServiceError::Run(described, source)),
        }
    }
}

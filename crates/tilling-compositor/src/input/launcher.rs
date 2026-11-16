use std::io;
use std::process::Child;

use tilling_config::CommandLine;

use crate::helpers::{SessionEnvironment, spawn};

#[derive(Debug)]
pub(crate) struct Launcher {
    environment: SessionEnvironment,
    children: Vec<Child>,
}

impl Launcher {
    pub(crate) fn new(environment: SessionEnvironment) -> Self {
        Self {
            environment,
            children: Vec::new(),
        }
    }

    pub(crate) fn launch(&mut self, command: &CommandLine) -> io::Result<()> {
        self.reap();
        let child = spawn(command, &self.environment, None)?;
        self.children.push(child);
        Ok(())
    }

    pub(crate) fn reap(&mut self) {
        self.children
            .retain_mut(|child| matches!(child.try_wait(), Ok(None)));
    }
}

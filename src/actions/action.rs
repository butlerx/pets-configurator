use super::{ActionError, Cause};
use std::{fmt, process::Command};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Action {
    cause: Cause,
    command: Vec<String>,
    requires_sudo: bool,
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.cause, self.command.join(" "))
    }
}

impl Action {
    pub fn new(cause: Cause, command: Vec<String>) -> Self {
        Self {
            cause,
            command,
            requires_sudo: false,
        }
    }

    pub fn use_sudo(self) -> Self {
        Self {
            requires_sudo: true,
            ..self
        }
    }

    pub fn with_sudo(cause: Cause, command: Vec<String>) -> Self {
        Self {
            cause,
            command,
            requires_sudo: true,
        }
    }

    pub fn perform(self, dry_run: bool) -> Result<i32, ActionError> {
        log::info!("Running {}", self.command.join(" "));
        if dry_run {
            return Ok(1);
        }

        let mut command = if self.requires_sudo {
            let mut cmd = Command::new("sudo");
            cmd.arg(&self.command[0]);
            cmd.args(&self.command[1..]);
            cmd
        } else {
            let mut cmd = Command::new(&self.command[0]);
            cmd.args(&self.command[1..]);
            cmd
        };

        if self.cause == Cause::Pkg {
            command.env("DEBIAN_FRONTEND", "noninteractive");
        }
        let output = match command.output() {
            Ok(output) => output,
            // if the package manger is not found, return Ok(0)
            // TODO filter these sooner
            Err(_) if self.cause == Cause::Pkg => return Ok(0),
            Err(err) => return Err(ActionError::IoError(err)),
        };

        if !output.stdout.is_empty() {
            log::info!(
                "{} => {}",
                self.command[0],
                String::from_utf8_lossy(&output.stdout)
            );
        }

        let status = output.status.code().unwrap_or(1);

        if !output.status.success() {
            let std_err = if output.stderr.is_empty() {
                "No error message".to_string()
            } else {
                String::from_utf8_lossy(&output.stderr).to_string()
            };

            return Err(ActionError::ExecError(
                self.command[0].clone(),
                status,
                std_err,
            ));
        }
        Ok(status)
    }
}

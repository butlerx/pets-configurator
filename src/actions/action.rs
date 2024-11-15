use super::{ActionError, Cause};
use std::{fmt, process::Command};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Action {
    cause: Cause,
    command: Vec<String>,
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.cause, self.command.join(" "))
    }
}

impl Action {
    pub fn new(cause: Cause, command: Vec<String>) -> Self {
        Action { cause, command }
    }

    pub fn perform(self, dry_run: bool) -> Result<i32, ActionError> {
        log::info!("Running {}", self.command.join(" "));
        if dry_run {
            return Ok(1);
        }

        let mut command = Command::new(&self.command[0]);
        command.args(&self.command[1..]);
        if self.cause == Cause::Pkg {
            command.env("DEBIAN_FRONTEND", "noninteractive");
        }
        let output = command.output()?;

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

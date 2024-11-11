use super::Cause;
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

    pub fn perform(self, dry_run: bool) -> std::io::Result<()> {
        log::info!("Running {}", self.command.join(" "));
        if dry_run {
            return Ok(());
        }

        let mut command = Command::new(&self.command[0]);
        command.args(&self.command[1..]);
        if self.cause == Cause::Pkg {
            command.env("DEBIAN_FRONTEND", "noninteractive");
        }
        let output = command.output()?;

        if !output.stdout.is_empty() {
            log::info!(
                "stdout from Perform() -> {:?}",
                String::from_utf8_lossy(&output.stdout)
            );
        }

        if !output.stderr.is_empty() {
            log::error!(
                "stderr from Perform() -> {:?}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }
}

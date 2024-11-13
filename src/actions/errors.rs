use thiserror::Error;

#[derive(Debug, Error)]
pub enum ActionError {
    #[error("Error executing command: {0}. Exit code: {1} {2}")]
    ExecError(String, i32, String),
    #[error("IO error running command: {0}")]
    IoError(#[from] std::io::Error),
}

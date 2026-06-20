use super::{ActionError, Cause};
use similar::TextDiff;
use std::{
    fmt, fs, io,
    os::unix::fs as unix_fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

pub struct RunConfig {
    pub dry_run: bool,
    pub backup: bool,
}

/// The underlying filesystem or system operation to perform.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Operation {
    /// Copy a file (or directory) from source to dest.
    Copy { source: PathBuf, dest: PathBuf },
    /// Create a symbolic link at `dest` pointing to `source`.
    Symlink { source: PathBuf, dest: PathBuf },
    /// Create a directory and its parents.
    CreateDir { path: PathBuf },
    /// Set file permissions (octal mode).
    Chmod { path: PathBuf, mode: u32 },
    /// Change file ownership, falling back to `sudo chown` on permission errors.
    Chown {
        path: PathBuf,
        uid: Option<u32>,
        gid: Option<u32>,
        /// Human-readable ownership string for display (e.g. "root:staff").
        display_arg: String,
        force_sudo: bool,
    },
    /// Run an arbitrary shell command.
    Command {
        args: Vec<String>,
        requires_sudo: bool,
    },
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Copy { source, dest } => {
                write!(f, "cp {} {}", source.display(), dest.display())
            }
            Self::Symlink { source, dest } => {
                write!(f, "ln -s {} {}", source.display(), dest.display())
            }
            Self::CreateDir { path } => write!(f, "mkdir -p {}", path.display()),
            Self::Chmod { path, mode } => write!(f, "chmod {mode:o} {}", path.display()),
            Self::Chown {
                path,
                display_arg,
                force_sudo,
                ..
            } => {
                if *force_sudo {
                    write!(f, "sudo chown {display_arg} {}", path.display())
                } else {
                    write!(f, "chown {display_arg} {}", path.display())
                }
            }
            Self::Command {
                args,
                requires_sudo,
            } => {
                if *requires_sudo {
                    write!(f, "sudo {}", args.join(" "))
                } else {
                    write!(f, "{}", args.join(" "))
                }
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Action {
    cause: Cause,
    operation: Operation,
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.cause, self.operation)
    }
}

impl Action {
    pub fn cause(&self) -> Cause {
        self.cause
    }

    pub fn copy_file(cause: Cause, source: PathBuf, dest: PathBuf) -> Self {
        Self {
            cause,
            operation: Operation::Copy { source, dest },
        }
    }

    pub fn symlink(cause: Cause, source: PathBuf, dest: PathBuf) -> Self {
        Self {
            cause,
            operation: Operation::Symlink { source, dest },
        }
    }

    pub fn create_dir(cause: Cause, path: PathBuf) -> Self {
        Self {
            cause,
            operation: Operation::CreateDir { path },
        }
    }

    pub fn chmod(cause: Cause, path: PathBuf, mode: u32) -> Self {
        Self {
            cause,
            operation: Operation::Chmod { path, mode },
        }
    }

    pub fn chown(
        cause: Cause,
        path: PathBuf,
        uid: Option<u32>,
        gid: Option<u32>,
        display_arg: String,
    ) -> Self {
        Self {
            cause,
            operation: Operation::Chown {
                path,
                uid,
                gid,
                display_arg,
                force_sudo: false,
            },
        }
    }

    pub fn command(cause: Cause, args: Vec<String>) -> Self {
        Self {
            cause,
            operation: Operation::Command {
                args,
                requires_sudo: false,
            },
        }
    }

    pub fn command_with_sudo(cause: Cause, args: Vec<String>) -> Self {
        Self {
            cause,
            operation: Operation::Command {
                args,
                requires_sudo: true,
            },
        }
    }

    pub fn use_sudo(mut self) -> Self {
        match &mut self.operation {
            Operation::Chown { force_sudo, .. } => *force_sudo = true,
            Operation::Command { requires_sudo, .. } => *requires_sudo = true,
            _ => log::warn!("use_sudo called on operation that doesn't support it"),
        }
        self
    }

    pub fn perform(self, config: &RunConfig) -> Result<i32, ActionError> {
        log::info!("{}", self.operation);

        if config.dry_run {
            self.log_dry_run_details()?;
            return Ok(0);
        }

        let Action { cause, operation } = self;

        match operation {
            Operation::Copy { source, dest } => {
                if config.backup && cause == Cause::Update && !source.is_dir() && dest.exists() {
                    let backup = backup_path_for(&dest);
                    fs::copy(&dest, &backup)?;
                    log::info!("backed up {} to {}", dest.display(), backup.display());
                }

                if source.is_dir() {
                    copy_dir_all(&source, &dest)?;
                } else {
                    atomic_copy(&source, &dest)?;
                }
                Ok(0)
            }
            Operation::Symlink { source, dest } => {
                unix_fs::symlink(&source, &dest)?;
                Ok(0)
            }
            Operation::CreateDir { path } => {
                fs::create_dir_all(&path)?;
                Ok(0)
            }
            Operation::Chmod { path, mode } => {
                fs::set_permissions(&path, fs::Permissions::from_mode(mode))?;
                Ok(0)
            }
            Operation::Chown {
                path,
                uid,
                gid,
                display_arg,
                force_sudo,
            } => {
                if force_sudo {
                    return sudo_chown(&display_arg, &path);
                }
                match unix_fs::chown(&path, uid, gid) {
                    Ok(()) => Ok(0),
                    Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
                        log::info!("chown requires elevated privileges, retrying with sudo");
                        sudo_chown(&display_arg, &path)
                    }
                    Err(e) => Err(e.into()),
                }
            }
            Operation::Command {
                args,
                requires_sudo,
            } => {
                let mut cmd = if requires_sudo {
                    let mut c = Command::new("sudo");
                    c.arg(&args[0]);
                    c.args(&args[1..]);
                    c
                } else {
                    let mut c = Command::new(&args[0]);
                    c.args(&args[1..]);
                    c
                };

                if cause == Cause::Pkg {
                    cmd.env("DEBIAN_FRONTEND", "noninteractive");
                }

                let output = match cmd.output() {
                    Ok(output) => output,
                    // if the package manager is not found, return Ok(0)
                    Err(_) if cause == Cause::Pkg => return Ok(0),
                    Err(err) => return Err(err.into()),
                };

                if !output.stdout.is_empty() {
                    log::info!("{} => {}", args[0], String::from_utf8_lossy(&output.stdout));
                }

                let status = output.status.code().unwrap_or(1);

                if !output.status.success() {
                    let std_err = if output.stderr.is_empty() {
                        "No error message".to_string()
                    } else {
                        String::from_utf8_lossy(&output.stderr).to_string()
                    };

                    return Err(ActionError::ExecError(args[0].clone(), status, std_err));
                }
                Ok(status)
            }
        }
    }

    fn log_dry_run_details(&self) -> Result<(), ActionError> {
        match (&self.cause, &self.operation) {
            (Cause::Create, Operation::Copy { source, .. }) => {
                log::info!("new file: {}", source.display());
            }
            (Cause::Update, Operation::Copy { source, dest }) => {
                log_unified_diff(source, dest)?;
            }
            _ => {}
        }

        Ok(())
    }
}

fn backup_path_for(dest: &Path) -> PathBuf {
    PathBuf::from(format!("{}.pets-backup", dest.to_string_lossy()))
}

fn log_unified_diff(source: &Path, dest: &Path) -> Result<(), ActionError> {
    let source_content = read_text_file(source)?;
    let dest_content = read_text_file(dest)?;

    match (source_content, dest_content) {
        (Some(source_text), Some(dest_text)) => {
            let from = dest.display().to_string();
            let to = source.display().to_string();
            let diff = TextDiff::from_lines(&dest_text, &source_text)
                .unified_diff()
                .header(&from, &to)
                .to_string();

            for line in diff.lines() {
                log::info!("{line}");
            }
        }
        _ => log::info!("binary file differs"),
    }

    Ok(())
}

fn read_text_file(path: &Path) -> Result<Option<String>, ActionError> {
    match fs::read_to_string(path) {
        Ok(content) => Ok(Some(content)),
        Err(err) if err.kind() == io::ErrorKind::InvalidData => Ok(None),
        Err(err) => Err(err.into()),
    }
}

/// Falls back to `sudo chown` when native chown fails with permission denied.
fn sudo_chown(arg: &str, path: &Path) -> Result<i32, ActionError> {
    let output = Command::new("sudo")
        .args(["chown", arg, &path.to_string_lossy()])
        .output()?;

    if output.status.success() {
        Ok(0)
    } else {
        let stderr = if output.stderr.is_empty() {
            "No error message".to_string()
        } else {
            String::from_utf8_lossy(&output.stderr).to_string()
        };
        Err(ActionError::ExecError(
            "sudo chown".to_string(),
            output.status.code().unwrap_or(1),
            stderr,
        ))
    }
}

/// Recursively copies a directory tree from `src` to `dst`.
fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let dest_path = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &dest_path)?;
        } else {
            fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}

/// Copies `source` to `dest` atomically by writing to a temporary file in the
/// same directory, then renaming. If the rename fails (e.g. cross-device), falls
/// back to a direct copy.
fn atomic_copy(source: &Path, dest: &Path) -> io::Result<()> {
    let tmp_path = PathBuf::from(format!("{}.pets-tmp", dest.to_string_lossy()));
    fs::copy(source, &tmp_path)?;
    if fs::rename(&tmp_path, dest).is_err() {
        let _ = fs::remove_file(&tmp_path);
        fs::copy(source, dest)?;
    }
    Ok(())
}

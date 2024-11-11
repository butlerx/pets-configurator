use super::{destination, mode, parser};
use crate::actions::{Cause, Package};
use std::{
    convert::TryFrom,
    fs,
    path::PathBuf,
    process::{Command, Stdio},
};

pub struct PetsFile {
    // Absolute path to the configuration file
    source: String,
    dest: destination::Destination,
    pkgs: Vec<Package>,
    user: Option<users::User>,
    group: Option<users::Group>,
    mode: mode::Mode,
    pre: Option<Vec<String>>,
    post: Option<Vec<String>>,
}

impl TryFrom<&PathBuf> for PetsFile {
    type Error = parser::ParseError;

    fn try_from(path: &PathBuf) -> Result<Self, Self::Error> {
        let modelines = parser::read_modelines(path)?;
        if modelines.is_empty() {
            return Err(parser::ParseError::NotPetsFile);
        }
        log::debug!("{} pets modelines found in {:?}", modelines.len(), path);
        //
        // Get absolute path to the source.
        let abs = fs::canonicalize(path)?;
        let source = abs.to_string_lossy().into_owned();

        let dest = match modelines.get("destfile") {
            Some(dest) => destination::Destination::from(dest),
            None => match modelines.get("symlink") {
                Some(dest) => destination::Destination::link(dest),
                None => return Err(parser::ParseError::MissingDestFile(source)),
            },
        };

        let mode = match modelines.get("mode") {
            Some(mode) => mode::Mode::try_from(mode)?,
            None => mode::Mode::default(),
        };

        let pkgs = match modelines.get("package") {
            Some(pkgs) => pkgs
                .split_whitespace()
                .map(|pkg| Package::new(pkg.to_string()))
                .collect(),
            None => Vec::new(),
        };

        let user = match modelines.get("owner") {
            Some(user) => {
                if let Some(user) = users::get_user_by_name(user) {
                    Some(user)
                } else {
                    // TODO: one day we may add support for creating users
                    log::error!("unknown 'owner' {}, skipping directive", user);
                    None
                }
            }
            None => users::get_user_by_uid(users::get_current_uid()),
        };

        let group = match modelines.get("group") {
            Some(group) => {
                if let Some(group) = users::get_group_by_name(group) {
                    Some(group)
                } else {
                    // TODO: one day we may add support for creating groups
                    log::error!("unknown 'group' {}, skipping directive", group);
                    None
                }
            }
            None => users::get_group_by_gid(users::get_current_gid()),
        };

        let pre = modelines
            .get("pre")
            .map(|pre| {
                let pre_args = pre
                    .split_whitespace()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<String>>();
                if pre_args.is_empty() {
                    None
                } else {
                    Some(pre_args)
                }
            })
            .unwrap_or_default();

        let post = modelines
            .get("post")
            .map(|post| {
                let post_args = post
                    .split_whitespace()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<String>>();
                if post_args.is_empty() {
                    None
                } else {
                    Some(post_args)
                }
            })
            .unwrap_or_default();

        log::debug!("'{:?}' pets syntax OK", path);
        Ok(Self {
            source,
            dest,
            pkgs,
            user,
            group,
            mode,
            pre,
            post,
        })
    }
}

impl PetsFile {
    pub fn destination(&self) -> &destination::Destination {
        &self.dest
    }

    pub fn source(&self) -> String {
        self.source.clone()
    }
    pub fn mode(&self) -> &mode::Mode {
        &self.mode
    }

    pub fn packages(&self) -> &[Package] {
        &self.pkgs
    }

    pub fn post(&self) -> Option<&Vec<String>> {
        self.post.as_ref()
    }

    pub fn user(&self) -> Option<&users::User> {
        self.user.as_ref()
    }

    pub fn group(&self) -> Option<&users::Group> {
        self.group.as_ref()
    }

    /// validates assumptions that must hold for the individual configuration files.
    /// Ignore `PathErrors` for now. Get a list of valid files.
    pub fn is_valid(&self) -> bool {
        log::debug!("validating {}", self.source);
        // Check if the specified package(s) exists
        for pkg in &self.pkgs {
            if !pkg.is_valid() {
                log::error!(
                    "Invalid configuration file, package {} not found for {}",
                    pkg,
                    self.source
                );
                return false;
            }
        }

        // Check pre-update validation command if the file has changed.
        if self.dest.needs_copy(&self.source) != Cause::None && !self.run_pre(true) {
            log::error!("pre-update validation failed for {}", self.source);
            false
        } else {
            log::debug!("{} is a valid configuration file", self.source);
            true
        }
    }

    // runPre returns true if the pre-update validation command passes, or if it
    // was not specified at all. The boolean argument pathErrorOK controls whether
    // or not we want to fail if the validation command is not around.
    fn run_pre(&self, path_error_ok: bool) -> bool {
        let Some(ref pre) = self.pre else {
            return true;
        };

        // Run 'pre' validation command, append Source filename to
        // arguments.
        // eg: /usr/sbin/sshd -t -f sample_pet/ssh/sshd_config
        let pre_command = Command::new(&pre[0])
            .args(&pre[1..])
            .arg(&self.source)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();

        match pre_command {
            Ok(output) => {
                log::info!("pre-update command {:?} successful", pre);
                if !output.stdout.is_empty() {
                    let stdout = std::str::from_utf8(&output.stdout)
                        .unwrap_or_default()
                        .to_string();

                    log::info!("stdout: {}", stdout);
                }
                if !output.stderr.is_empty() {
                    let stderr = std::str::from_utf8(&output.stderr)
                        .unwrap_or_default()
                        .to_string();
                    log::error!("stderr: {}", stderr);
                }
                true
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound && path_error_ok => {
                // The command has failed because the validation command itself is
                // missing. This could be a chicken-and-egg problem: at this stage
                // configuration is not validated yet, hence any "package" directives
                // have not been applied.  Do not consider this as a failure, for now.
                log::info!(
                    "pre-update command {:?} failed due to PathError. Ignoring for now",
                    pre
                );
                true
            }
            Err(err) => {
                log::error!("pre-update command {:?}: {}\n", pre, err);
                false
            }
        }
    }
}

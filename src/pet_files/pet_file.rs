use super::{condition::Condition, destination, mode, parser};
use crate::actions::{Action, ActionError, Cause, Package, package_manager::PackageManager};
use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::PathBuf,
    process::{Command, Stdio},
};

pub enum SyncStatus {
    InSync,
    Missing,
    Modified,
    LinkMissing,
    LinkWrong,
}

pub struct PetsFile {
    // Absolute path to the configuration file
    source: String,
    dest: destination::Destination,
    pkgs: Vec<Package>,
    user: Option<uzers::User>,
    group: Option<uzers::Group>,
    mode: mode::Mode,
    pre: Option<Vec<String>>,
    post: Option<Vec<String>>,
    conditions: Vec<Condition>,
}

impl PetsFile {
    pub fn from_path(
        path: &PathBuf,
        package_manager: PackageManager,
    ) -> Result<Self, parser::ParseError> {
        let modelines = parser::read_modelines(path)?;
        if modelines.is_empty() {
            return Err(parser::ParseError::NotPetsFile);
        }
        log::debug!(
            "{} pets modelines found in {}",
            modelines.len(),
            path.display()
        );

        // Get absolute path to the source.
        let abs = fs::canonicalize(path)?;
        let source = abs.to_string_lossy().into_owned();
        let is_petsfile = match abs.file_name() {
            Some(file_name) => file_name.to_string_lossy().to_lowercase() == ".petsfile",
            _ => false,
        };

        let dest = match modelines.get("destfile") {
            Some(dest) => destination::Destination::new(&dest[0], false, is_petsfile),
            None => match modelines.get("symlink") {
                Some(dest) => destination::Destination::new(&dest[0], true, is_petsfile),
                None => return Err(parser::ParseError::MissingDestFile(source)),
            },
        };

        let mode = match modelines.get("mode") {
            Some(mode) => mode::Mode::try_from(&mode[0])?,
            None => mode::Mode::default(),
        };

        let pkgs = match modelines.get("package") {
            Some(pkgs) => pkgs
                .iter()
                .map(|pkg| Package::new(pkg, package_manager))
                .collect(),
            None => Vec::new(),
        };

        let user = match modelines.get("owner") {
            Some(user) => {
                if let Some(user) = uzers::get_user_by_name(&user[0]) {
                    Some(user)
                } else {
                    // TODO: one day we may add support for creating users
                    log::warn!("unknown 'owner' {}, skipping directive", user[0]);
                    uzers::get_user_by_uid(uzers::get_current_uid())
                }
            }
            None => uzers::get_user_by_uid(uzers::get_current_uid()),
        };

        let group = match modelines.get("group") {
            Some(group) => {
                if let Some(group) = uzers::get_group_by_name(&group[0]) {
                    Some(group)
                } else {
                    // TODO: one day we may add support for creating groups
                    log::warn!("unknown 'group' {}, skipping directive", &group[0]);
                    uzers::get_group_by_gid(uzers::get_current_gid())
                }
            }
            None => uzers::get_group_by_gid(uzers::get_current_gid()),
        };

        let pre = parse_command_directive(modelines.get("pre"));
        let post = parse_command_directive(modelines.get("post"));
        let conditions = parse_conditions(modelines.get("when"))?;

        log::debug!("'{}' pets syntax OK", path.display());
        Ok(Self {
            source,
            dest,
            pkgs,
            user,
            group,
            mode,
            pre,
            post,
            conditions,
        })
    }

    pub fn destination(&self) -> String {
        self.dest.to_string()
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn packages(&self) -> &[Package] {
        &self.pkgs
    }

    pub fn is_symlink_config(&self) -> bool {
        self.dest.is_symlink()
    }

    pub fn sync_status(&self) -> SyncStatus {
        if self.dest.is_symlink() {
            match self.dest.needs_link(&self.source) {
                None => SyncStatus::InSync,
                Some(action) => match action.cause() {
                    Cause::Link => SyncStatus::LinkMissing,
                    _ => SyncStatus::LinkWrong,
                },
            }
        } else {
            match self.dest.needs_copy(&self.source) {
                None => SyncStatus::InSync,
                Some(action) => match action.cause() {
                    Cause::Create => SyncStatus::Missing,
                    _ => SyncStatus::Modified,
                },
            }
        }
    }

    pub fn matches_conditions(&self) -> bool {
        self.conditions.iter().all(Condition::is_met)
    }

    /// validates assumptions that must hold for the individual configuration files.
    /// Ignore `PathErrors` for now. Get a list of valid files.
    pub fn is_valid(&self) -> bool {
        log::debug!("validating {}", self.source);
        // Check if the specified package(s) exists
        for pkg in &self.pkgs {
            match pkg.is_valid() {
                Ok(()) | Err(ActionError::NoPackageManager) => {}
                Err(err) => {
                    log::error!("Invalid configuration file, {err}");
                    return false;
                }
            }
        }

        // Check pre-update validation command if the file has changed.
        if self.dest.needs_copy(&self.source).is_some() && !self.run_pre(true) {
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
            Ok(output) if output.status.success() => {
                log::info!("pre-update command {pre:?} successful");
                if !output.stdout.is_empty() {
                    let stdout = std::str::from_utf8(&output.stdout)
                        .unwrap_or_default()
                        .to_string();

                    log::info!("stdout: {stdout}");
                }
                if !output.stderr.is_empty() {
                    let stderr = std::str::from_utf8(&output.stderr)
                        .unwrap_or_default()
                        .to_string();
                    log::warn!("stderr: {stderr}");
                }
                true
            }
            Ok(output) => {
                let stderr = if output.stderr.is_empty() {
                    "no error output".to_string()
                } else {
                    std::str::from_utf8(&output.stderr)
                        .unwrap_or_default()
                        .to_string()
                };
                log::error!(
                    "pre-update command {pre:?} failed with status {}: {stderr}",
                    output.status
                );
                false
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound && path_error_ok => {
                // The command has failed because the validation command itself is
                // missing. This could be a chicken-and-egg problem: at this stage
                // configuration is not validated yet, hence any "package" directives
                // have not been applied.  Do not consider this as a failure, for now.
                log::info!("pre-update command {pre:?} failed due to PathError. Ignoring for now");
                true
            }
            Err(err) => {
                log::error!("pre-update command {pre:?}: {err}\n");
                false
            }
        }
    }

    /// returns a chown `Action` or nil if none is needed.
    #[allow(clippy::similar_names)]
    fn chown(&self) -> Option<Action> {
        // Build arg (eg: 'root:staff', 'root', ':staff')
        let mut arg = String::new();

        let want_user_id = match &self.user {
            Some(user) => {
                let user_name = match user.name().to_str() {
                    Some(name) => name,
                    None => &user.name().to_string_lossy(),
                };
                arg.push_str(user_name);
                Some(user.uid())
            }
            None => None,
        };

        let want_group_id = match &self.group {
            Some(group) => {
                if !arg.is_empty() {
                    arg.push(':');
                }
                let group_name = match group.name().to_str() {
                    Some(name) => name,
                    None => &group.name().to_string_lossy(),
                };
                arg.push_str(group_name);

                // Get the requested gid as integer
                Some(group.gid())
            }
            None => None,
        };

        if arg.is_empty() {
            return None;
        }

        let action = Action::chown(
            Cause::Owner,
            PathBuf::from(self.dest.to_string()),
            want_user_id,
            want_group_id,
            arg,
        );

        let destination = self.dest.to_string();
        // stat() the destination file to see if a chown is needed
        let stat = match fs::metadata(&destination) {
            Ok(info) => info,
            Err(e) => match e.kind() {
                std::io::ErrorKind::NotFound => {
                    // If the destination file is not there yet, prepare a chown for later on.
                    return Some(action);
                }
                std::io::ErrorKind::PermissionDenied => {
                    log::error!("permission denied in chown(): {e}");
                    return Some(action.use_sudo());
                }
                _ => {
                    log::error!("unexpected error in chown(): {e}");
                    return None;
                }
            },
        };

        let file_uid = stat.uid();
        let file_gid = stat.gid();

        // Get the file ownership details from the metadata
        if let Some(want_uid) = want_user_id {
            if file_uid != want_uid {
                log::info!("{destination} is owned by uid {file_uid} instead of {want_uid}");
                return Some(action);
            }
        }

        if let Some(want_gid) = want_group_id {
            if file_gid != want_gid {
                log::info!("{destination} is owned by gid {file_gid} instead of {want_gid}");
                return Some(action);
            }
        }

        log::debug!("{destination} is owned by {file_uid}:{file_gid} already");
        None
    }

    ///  returns a chmod `Action` or nil if none is needed.
    fn chmod(&self) -> Option<Action> {
        if self.mode.is_empty() {
            return None;
        }

        let action = Action::chmod(
            Cause::Mode,
            PathBuf::from(self.dest.to_string()),
            self.mode.as_raw(),
        );

        // stat(2) the destination file to see if a chmod is needed
        let file_info = match fs::metadata(self.dest.to_string()) {
            Ok(info) => info,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Some(action);
            }
            Err(e) => {
                log::error!("unexpected error in chmod(): {e}");
                return None;
            }
        };

        let old_mode = file_info.permissions().mode();

        // See if the desired mode and reality differ.
        if self.mode == old_mode {
            log::debug!("{} is {} already", self.dest, self.mode);
            None
        } else {
            log::info!("{} is {:o} instead of {}", self.dest, old_mode, self.mode);
            Some(action)
        }
    }
}

fn parse_command_directive(directive: Option<&Vec<String>>) -> Option<Vec<String>> {
    directive
        .and_then(|values| values.first())
        .map(|value| {
            value
                .split_whitespace()
                .map(std::string::ToString::to_string)
                .collect::<Vec<String>>()
        })
        .filter(|args| !args.is_empty())
}

fn parse_conditions(
    conditions: Option<&Vec<String>>,
) -> Result<Vec<Condition>, parser::ParseError> {
    conditions
        .map(|when| when.iter().map(|value| Condition::parse(value)).collect())
        .transpose()
        .map(Option::unwrap_or_default)
}

impl From<&PetsFile> for Vec<Action> {
    fn from(val: &PetsFile) -> Self {
        log::debug!("planning actions for {}", val.source);
        let actions = vec![
            val.dest.needs_dir(),
            val.dest.needs_copy(&val.source),
            val.dest.needs_link(&val.source),
            val.chown(),
            val.chmod(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
        // If any actions are performed, check for a post-action
        if actions.is_empty() {
            actions
        } else {
            let post = val
                .post
                .as_ref()
                .map(|post| Action::command(Cause::Post, post.clone()));
            actions.into_iter().chain(post).collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::package_manager;
    use std::io::Write;
    use tempfile::tempdir;

    fn write_pets_file(path: &PathBuf, lines: &[&str], body: &str) {
        let mut file = std::fs::File::create(path).unwrap();
        for line in lines {
            writeln!(file, "{line}").unwrap();
        }
        writeln!(file, "{body}").unwrap();
    }

    fn package_manager_for_tests() -> PackageManager {
        package_manager::which().unwrap()
    }

    #[test]
    fn test_from_path_parses_destfile_directive() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("config.conf");
        let dest = dir.path().join("dest.conf");
        write_pets_file(
            &source,
            &[&format!("# pets: destfile={}", dest.display())],
            "value=true",
        );

        let parsed = PetsFile::from_path(&source, package_manager_for_tests()).unwrap();
        assert_eq!(parsed.destination(), dest.to_string_lossy());
    }

    #[test]
    fn test_from_path_parses_symlink_directive() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("config.conf");
        let link_path = dir.path().join("config-link");
        write_pets_file(
            &source,
            &[&format!("# pets: symlink={}", link_path.display())],
            "value=true",
        );

        let parsed = PetsFile::from_path(&source, package_manager_for_tests()).unwrap();
        assert!(parsed.dest.is_symlink());
        assert_eq!(parsed.destination(), link_path.to_string_lossy());
    }

    #[test]
    fn test_from_path_missing_destfile_or_symlink_returns_error() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("config.conf");
        write_pets_file(&source, &["# pets: package=exa"], "value=true");

        let parsed = PetsFile::from_path(&source, package_manager_for_tests());
        assert!(matches!(
            parsed,
            Err(parser::ParseError::MissingDestFile(_))
        ));
    }

    #[test]
    fn test_from_path_parses_mode_directive() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("config.conf");
        let dest = dir.path().join("dest.conf");
        write_pets_file(
            &source,
            &[
                &format!("# pets: destfile={}", dest.display()),
                "# pets: mode=0640",
            ],
            "value=true",
        );

        let parsed = PetsFile::from_path(&source, package_manager_for_tests()).unwrap();
        assert_eq!(parsed.mode.as_raw(), 0o640);
    }

    #[test]
    fn test_from_path_parses_package_directive() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("config.conf");
        let dest = dir.path().join("dest.conf");
        write_pets_file(
            &source,
            &[
                &format!("# pets: destfile={}", dest.display()),
                "# pets: package=exa",
                "# pets: package=cargo:bat",
            ],
            "value=true",
        );

        let parsed = PetsFile::from_path(&source, package_manager_for_tests()).unwrap();
        let packages = parsed.packages();
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].name, "exa");
        assert_eq!(packages[1].name, "bat");
        assert_eq!(packages[1].package_manager, PackageManager::Cargo);
    }

    #[test]
    fn test_from_path_parses_owner_group_directives() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("config.conf");
        let dest = dir.path().join("dest.conf");
        let current_user = uzers::get_user_by_uid(uzers::get_current_uid()).unwrap();
        let current_group = uzers::get_group_by_gid(uzers::get_current_gid()).unwrap();
        let user_name = current_user.name().to_string_lossy().into_owned();
        let group_name = current_group.name().to_string_lossy().into_owned();

        write_pets_file(
            &source,
            &[
                &format!("# pets: destfile={}", dest.display()),
                &format!("# pets: owner={user_name}"),
                &format!("# pets: group={group_name}"),
            ],
            "value=true",
        );

        let parsed = PetsFile::from_path(&source, package_manager_for_tests()).unwrap();
        assert_eq!(
            parsed.user.as_ref().map(uzers::User::uid),
            Some(current_user.uid())
        );
        assert_eq!(
            parsed.group.as_ref().map(uzers::Group::gid),
            Some(current_group.gid())
        );
    }

    #[test]
    fn test_from_path_unknown_owner_falls_back_to_current_user() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("config.conf");
        let dest = dir.path().join("dest.conf");
        let current_uid = uzers::get_current_uid();
        write_pets_file(
            &source,
            &[
                &format!("# pets: destfile={}", dest.display()),
                "# pets: owner=definitely_unknown_user_name_12345",
            ],
            "value=true",
        );

        let parsed = PetsFile::from_path(&source, package_manager_for_tests()).unwrap();
        assert_eq!(
            parsed.user.as_ref().map(uzers::User::uid),
            Some(current_uid)
        );
    }

    #[test]
    fn test_from_path_parses_pre_post_and_when_directives() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("config.conf");
        let dest = dir.path().join("dest.conf");
        let current_os = if cfg!(target_os = "macos") {
            "macos"
        } else {
            "linux"
        };

        write_pets_file(
            &source,
            &[
                &format!("# pets: destfile={}", dest.display()),
                "# pets: pre=/usr/bin/true --check",
                "# pets: post=/bin/echo reloaded",
                &format!("# pets: when=os:{current_os}"),
            ],
            "value=true",
        );

        let parsed = PetsFile::from_path(&source, package_manager_for_tests()).unwrap();
        assert_eq!(
            parsed.pre,
            Some(vec!["/usr/bin/true".to_string(), "--check".to_string()])
        );
        assert_eq!(
            parsed.post,
            Some(vec!["/bin/echo".to_string(), "reloaded".to_string()])
        );
        assert_eq!(
            parsed.conditions,
            vec![Condition::Os(current_os.to_string())]
        );
        assert!(parsed.matches_conditions());
    }

    #[test]
    fn test_getters_and_validity_with_no_conditions() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("config.conf");
        let dest = dir.path().join("dest.conf");
        write_pets_file(
            &source,
            &[&format!("# pets: destfile={}", dest.display())],
            "same-content",
        );

        let parsed = PetsFile::from_path(&source, package_manager_for_tests()).unwrap();
        let expected_source = std::fs::canonicalize(&source)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert_eq!(parsed.source(), expected_source);
        assert_eq!(parsed.destination(), dest.to_string_lossy());
        assert!(parsed.packages().is_empty());
        assert!(parsed.matches_conditions());
        assert!(parsed.is_valid());
    }

    #[test]
    fn test_actions_from_pets_file_generates_copy_action() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("copy_source.conf");
        let dest = dir.path().join("copy_dest.conf");
        write_pets_file(
            &source,
            &[&format!("# pets: destfile={}", dest.display())],
            "copy-content",
        );

        let mut parsed = PetsFile::from_path(&source, package_manager_for_tests()).unwrap();
        parsed.user = None;
        parsed.group = None;

        let source_abs = std::fs::canonicalize(&source).unwrap();
        let actions: Vec<Action> = (&parsed).into();
        assert_eq!(
            actions,
            vec![Action::copy_file(Cause::Create, source_abs, dest.clone())]
        );
    }

    #[test]
    fn test_actions_from_pets_file_generates_symlink_action() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("link_source.conf");
        let dest = dir.path().join("link_dest");
        write_pets_file(
            &source,
            &[&format!("# pets: symlink={}", dest.display())],
            "link-content",
        );

        let mut parsed = PetsFile::from_path(&source, package_manager_for_tests()).unwrap();
        parsed.user = None;
        parsed.group = None;

        let source_abs = std::fs::canonicalize(&source).unwrap();
        let actions: Vec<Action> = (&parsed).into();
        assert_eq!(
            actions,
            vec![Action::symlink(Cause::Link, source_abs, dest)]
        );
    }

    #[test]
    fn test_actions_from_pets_file_appends_post_after_file_actions() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("post_source.conf");
        let dest = dir.path().join("post_dest.conf");
        write_pets_file(
            &source,
            &[
                &format!("# pets: destfile={}", dest.display()),
                "# pets: post=/bin/echo done",
            ],
            "copy-content",
        );

        let mut parsed = PetsFile::from_path(&source, package_manager_for_tests()).unwrap();
        parsed.user = None;
        parsed.group = None;

        let source_abs = std::fs::canonicalize(&source).unwrap();
        let actions: Vec<Action> = (&parsed).into();
        assert_eq!(actions.len(), 2);
        assert_eq!(
            actions[0],
            Action::copy_file(Cause::Create, source_abs, dest.clone())
        );
        assert_eq!(
            actions[1],
            Action::command(
                Cause::Post,
                vec!["/bin/echo".to_string(), "done".to_string()]
            )
        );
    }

    #[test]
    fn test_actions_from_pets_file_in_sync_generates_no_actions() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("sync_source.conf");
        let dest = dir.path().join("sync_dest.conf");
        write_pets_file(
            &source,
            &[&format!("# pets: destfile={}", dest.display())],
            "identical-content",
        );

        std::fs::copy(&source, &dest).unwrap();

        let mut parsed = PetsFile::from_path(&source, package_manager_for_tests()).unwrap();
        parsed.user = None;
        parsed.group = None;

        let actions: Vec<Action> = (&parsed).into();
        assert!(actions.is_empty());
    }
}

use super::{package_manager, Cause};
use crate::pet_files::PetsFile;
use std::{
    collections::HashSet,
    fmt, fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    process::Command,
};

#[derive(Debug)]
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

/// Determines a list of packages that need to be installed.
pub fn pkgs_to_install(triggers: &[PetsFile]) -> Vec<package_manager::Package> {
    let mut pkgs = HashSet::new();

    for trigger in triggers {
        for pkg in trigger.packages() {
            if pkg.is_installed() {
                log::debug!("{} already installed", pkg);
            } else {
                log::info!("{} not installed", pkg);
                pkgs.insert(pkg.clone());
            }
        }
    }

    pkgs.into_iter().collect()
}

/// figures out if the given trigger represents a file that needs to
/// be updated, and returns the corresponding `Action`.
fn file_to_copy(trigger: &PetsFile) -> Option<Action> {
    match trigger.destination().needs_copy(&trigger.source()) {
        Cause::None => None,
        cause => Some(Action::new(
            cause,
            vec![
                String::from("cp"),
                trigger.source().to_string(),
                trigger.destination().to_string(),
            ],
        )),
    }
}

/// figures out if the given trigger represents a symbolic link
/// that needs to be created, and returns the corresponding `Action`.
fn link_to_create(trigger: &PetsFile) -> Option<Action> {
    match trigger.destination().needs_link(&trigger.source()) {
        Cause::None => None,
        cause => Some(Action::new(
            cause,
            vec![
                String::from("ln"),
                String::from("-s"),
                trigger.source().to_string(),
                trigger.destination().to_string(),
            ],
        )),
    }
}

/// figures out if the given trigger represents a directory that
/// needs to be created, and returns the corresponding `Action`.
fn dir_to_create(trigger: &PetsFile) -> Option<Action> {
    match trigger.destination().needs_dir() {
        Cause::None => None,
        cause => Some(Action::new(
            cause,
            vec![
                String::from("mkdir"),
                String::from("-p"),
                trigger.destination().directory(),
            ],
        )),
    }
}

///returns a chown `Action` or nil if none is needed.
fn chown(trigger: &PetsFile) -> Option<Action> {
    // Build arg (eg: 'root:staff', 'root', ':staff')
    let mut arg = String::new();

    let want_user_id = match trigger.user() {
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

    let want_group_id = match trigger.group() {
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
        // Return immediately if the file had no 'owner' / 'group' directives
        return None;
    }

    // The action to (possibly) perform is a chown of the file.
    let action = Action {
        cause: Cause::Owner,
        command: vec![
            "/bin/chown".to_string(),
            arg.clone(),
            trigger.destination().to_string(),
        ],
    };

    let destination = trigger.destination().to_string();
    // stat() the destination file to see if a chown is needed
    let file_info = match fs::metadata(&destination) {
        Ok(info) => info,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // If the destination file is not there yet, prepare a chown for later on.
            return Some(action);
        }
        Err(e) => {
            log::error!("unexpected error in chown(): {}", e);
            return None;
        }
    };

    // Get the file ownership details from the metadata
    let stat = file_info;

    if let Some(want_uid) = want_user_id {
        if stat.uid() != want_uid {
            log::info!(
                "{} is owned by uid {} instead of {}",
                destination,
                stat.uid(),
                want_uid
            );
            return Some(action);
        }
    }

    if let Some(want_gid) = want_group_id {
        if stat.gid() != want_gid {
            log::info!(
                "{} is owned by gid {} instead of {}",
                destination,
                stat.gid(),
                want_gid
            );
            return Some(action);
        }
    }

    log::debug!(
        "{} is owned by {}:{} already",
        destination,
        stat.uid(),
        stat.gid()
    );
    None
}

///  returns a chmod `Action` or nil if none is needed.
fn chmod(trigger: &PetsFile) -> Option<Action> {
    if trigger.mode().is_empty() {
        return None;
    }

    let action = Action {
        cause: Cause::Mode,
        command: vec![
            "/bin/chmod".to_string(),
            trigger.mode().to_string(),
            trigger.destination().to_string(),
        ],
    };

    // stat(2) the destination file to see if a chmod is needed
    let file_info = match fs::metadata(trigger.destination().to_string()) {
        Ok(info) => info,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Some(action);
        }
        Err(e) => {
            log::error!("unexpected error in chmod(): {}", e);
            return None;
        }
    };

    let old_mode = file_info.permissions().mode();

    // See if the desired mode and reality differ.
    match trigger.mode().as_u32() {
        Ok(new_mode) if old_mode == new_mode => {
            log::info!("{} is {:o} already", trigger.destination(), new_mode);
            None
        }
        Ok(new_mode) => {
            log::info!(
                "{} is {:o} instead of {:o}",
                trigger.destination(),
                old_mode,
                new_mode
            );
            Some(action)
        }
        Err(e) => {
            log::error!("unexpected error in chmod(): {}", e);
            None
        }
    }
}

pub fn plan(triggers: &[PetsFile], family: &package_manager::PackageManager) -> Vec<Action> {
    let install_actions = pkgs_to_install(triggers)
        .is_empty()
        .then(|| package_manager::install_command(family))
        .map(|install_command| Action::new(Cause::Pkg, install_command))
        .into_iter();

    let trigger_actions = triggers.iter().flat_map(|trigger| {
        let actions = vec![
            dir_to_create(trigger),
            file_to_copy(trigger),
            link_to_create(trigger),
            chown(trigger),
            chmod(trigger),
        ]
        .into_iter()
        .flatten() // Flatten Option<T> to just T for non-None values
        .collect::<Vec<_>>();

        // If any actions are performed, check for a post-action
        if actions.is_empty() {
            actions
        } else {
            trigger
                .post()
                .map(|post| Action::new(Cause::Post, post.clone()))
                .into_iter()
                .chain(actions)
                .collect()
        }
    });

    install_actions.chain(trigger_actions).collect()
}

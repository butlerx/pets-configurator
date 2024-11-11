use crate::{package_manager, pet_files::PetsFile};
use std::{
    collections::HashSet,
    fmt, fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    process::Command,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PetsCause {
    None,
    Pkg,
    Create,
    Update,
    Link,
    Dir,
    Owner,
    Mode,
    Post,
}

impl fmt::Display for PetsCause {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let pets_cause = match self {
            PetsCause::Pkg => "PACKAGE_INSTALL",
            PetsCause::Create => "FILE_CREATE",
            PetsCause::Update => "FILE_UPDATE",
            PetsCause::Link => "LINK_CREATE",
            PetsCause::Dir => "DIR_CREATE",
            PetsCause::Owner => "OWNER",
            PetsCause::Mode => "CHMOD",
            PetsCause::Post => "POST_UPDATE",
            PetsCause::None => "NONE",
        };

        write!(f, "{}", pets_cause)
    }
}

#[derive(Debug)]
pub struct PetsAction {
    cause: PetsCause,
    command: Vec<String>,
}

impl fmt::Display for PetsAction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.cause, self.command.join(" "))
    }
}

impl PetsAction {
    pub fn new(cause: PetsCause, command: Vec<String>) -> Self {
        PetsAction { cause, command }
    }

    pub fn perform(self, dry_run: bool) -> std::io::Result<()> {
        log::info!("Running {}", self.command.join(" "));
        if dry_run {
            return Ok(());
        }

        let mut command = Command::new(&self.command[0]);
        command.args(&self.command[1..]);
        if self.cause == PetsCause::Pkg {
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
pub fn pkgs_to_install(triggers: &[PetsFile]) -> Vec<package_manager::PetsPackage> {
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
/// be updated, and returns the corresponding PetsAction.
fn file_to_copy(trigger: &PetsFile) -> Option<PetsAction> {
    match trigger.destination().needs_copy(&trigger.source()) {
        PetsCause::None => None,
        cause => Some(PetsAction::new(
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
/// that needs to be created, and returns the corresponding PetsAction.
fn link_to_create(trigger: &PetsFile) -> Option<PetsAction> {
    match trigger.destination().needs_link(trigger.source()) {
        PetsCause::None => None,
        cause => Some(PetsAction::new(
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
/// needs to be created, and returns the corresponding PetsAction.
fn dir_to_create(trigger: &PetsFile) -> Option<PetsAction> {
    match trigger.destination().needs_dir() {
        PetsCause::None => None,
        cause => Some(PetsAction::new(
            cause,
            vec![
                String::from("mkdir"),
                String::from("-p"),
                trigger.destination().directory(),
            ],
        )),
    }
}

///returns a chown PetsAction or nil if none is needed.
fn chown(trigger: &PetsFile) -> Option<PetsAction> {
    // Build arg (eg: 'root:staff', 'root', ':staff')
    let mut arg = String::new();

    let want_uid = match trigger.user() {
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

    let want_gid = match trigger.group() {
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
    let action = PetsAction {
        cause: PetsCause::Owner,
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

    if let Some(want_uid) = want_uid {
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

    if let Some(want_gid) = want_gid {
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

///  returns a chmod PetsAction or nil if none is needed.
fn chmod(trigger: &PetsFile) -> Option<PetsAction> {
    if trigger.mode().is_empty() {
        return None;
    }

    let action = PetsAction {
        cause: PetsCause::Mode,
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

pub fn plan(triggers: &[PetsFile], family: package_manager::PackageManager) -> Vec<PetsAction> {
    let mut actions = Vec::new();

    let pkgs = pkgs_to_install(triggers);
    if !pkgs.is_empty() {
        let install_command = package_manager::install_command(family);
        actions.push(PetsAction::new(PetsCause::Pkg, install_command));
    }

    for trigger in triggers {
        let mut run_post = false;
        if let Some(action) = dir_to_create(trigger) {
            actions.push(action);
            run_post = true;
        }
        if let Some(action) = file_to_copy(trigger) {
            actions.push(action);
            run_post = true;
        }
        if let Some(action) = link_to_create(trigger) {
            actions.push(action);
            run_post = true;
        }
        if let Some(action) = chown(trigger) {
            actions.push(action);
            run_post = true;
        }
        if let Some(action) = chmod(trigger) {
            actions.push(action);
            run_post = true;
        }
        if run_post {
            if let Some(post) = trigger.post() {
                actions.push(PetsAction::new(PetsCause::Post, post.clone()));
            }
        }
    }
    actions
}

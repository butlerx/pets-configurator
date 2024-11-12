// Pets configuration file validator and planner. Given a list of in-memory PetsFile(s),
// see if our sanity constraints are met. For example, we do not want multiple
// files to be installed to the same destination path. Also, all validation
// commands must succeed.

use crate::{
    actions::{self, package_manager::PackageManager},
    pet_files::PetsFile,
};
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
    string::ToString,
};

#[derive(Debug)]
pub struct DuplicateDefinitionError {
    dest: String,
    source: String,
    other: String,
}

impl DuplicateDefinitionError {
    pub fn new(dest: String, source: String, other: String) -> Self {
        Self {
            dest,
            source,
            other,
        }
    }
}

impl std::fmt::Display for DuplicateDefinitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "duplicate definition for '{}': '{}' and '{}'",
            self.dest, self.source, self.other
        )
    }
}

/// validates assumptions that must hold across all configuration files.
pub fn check_global_constraints(files: &[PetsFile]) -> Result<(), DuplicateDefinitionError> {
    let mut seen: HashMap<String, &PetsFile> = HashMap::new();

    for pf in files {
        let dest = pf.destination();
        if let Some(other) = seen.get(&dest) {
            return Err(DuplicateDefinitionError::new(
                dest,
                pf.source(),
                other.source(),
            ));
        }
        seen.insert(dest.to_string(), pf);
    }

    Ok(())
}

pub fn plan_actions(files: Vec<PetsFile>) -> Vec<actions::Action> {
    // Check validation errors in individual files. At this stage, the
    // command in the "pre" validation directive may not be installed yet.
    // An error in one file means we're gonna skip it but proceed with the rest.
    let good_pets = files
        .into_iter()
        .filter(PetsFile::is_valid)
        .collect::<Vec<_>>();

    // Determines a list of packages that need to be installed.
    let pkgs = good_pets
        .iter()
        .flat_map(|trigger| {
            trigger
                .packages()
                .iter()
                .filter(|pkg| !pkg.is_installed())
                .map(std::clone::Clone::clone)
        })
        .collect::<HashSet<actions::package_manager::Package>>();

    // Generate the list of actions to perform.
    let trigger_actions = good_pets
        .iter()
        .flat_map(Into::<Vec<actions::Action>>::into);
    if pkgs.is_empty() {
        trigger_actions.collect()
    } else {
        let mut packages: HashMap<String, Vec<String>> = HashMap::new();
        for pkg in pkgs {
            packages
                .entry(pkg.package_manager.to_string())
                .or_default()
                .push(pkg.name);
        }
        packages
            .iter()
            .map(|(pkg_manager, packages)| {
                let pkg_manager = PackageManager::from_str(pkg_manager).unwrap();
                let install_vec = pkg_manager.install_command();
                actions::Action::new(
                    actions::Cause::Pkg,
                    install_vec
                        .into_iter()
                        .chain(packages.iter().map(ToString::to_string))
                        .collect(),
                )
            })
            .chain(trigger_actions)
            .collect()
    }
}

use crate::{
    manifest::{Manifest, ResourceStatus},
    pet_files,
};
use colored::Colorize;
use std::{path::Path, process::ExitCode};

pub fn list(conf_dir: &str) -> ExitCode {
    let root = Path::new(conf_dir);
    let files = match pet_files::load(root) {
        Ok(files) => files,
        Err(err) => {
            log::error!("{err}");
            return ExitCode::FAILURE;
        }
    };
    let resources = match Manifest::load(root).and_then(|manifest| {
        manifest.map_or_else(
            || Ok(Vec::new()),
            |manifest| manifest.resource_statuses(root),
        )
    }) {
        Ok(resources) => resources,
        Err(err) => {
            log::error!("{err}");
            return ExitCode::FAILURE;
        }
    };

    if files.is_empty() && resources.is_empty() {
        println!("No pets configuration files or manifest resources found");
        return ExitCode::SUCCESS;
    }

    let files_in_sync = files
        .iter()
        .map(print_file_status)
        .reduce(|all, in_sync| all && in_sync)
        .unwrap_or(true);
    let resources_in_sync = resources
        .iter()
        .map(print_resource_status)
        .reduce(|all, in_sync| all && in_sync)
        .unwrap_or(true);

    if files_in_sync && resources_in_sync {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn print_file_status(file: &pet_files::PetsFile) -> bool {
    let destination = file.destination();
    let kind = if file.is_symlink_config() {
        "symlink"
    } else {
        "destfile"
    };

    match file.sync_status() {
        pet_files::SyncStatus::InSync => {
            println!("{} {} ({kind}, in sync)", "✓".green(), destination);
            true
        }
        pet_files::SyncStatus::Missing | pet_files::SyncStatus::LinkMissing => {
            println!("{} {} ({kind}, missing)", "✗".red(), destination);
            false
        }
        pet_files::SyncStatus::Modified => {
            println!("{} {} ({kind}, modified)", "~".yellow(), destination);
            false
        }
        pet_files::SyncStatus::LinkWrong => {
            println!("{} {} ({kind}, wrong target)", "~".yellow(), destination);
            false
        }
    }
}

fn print_resource_status(resource: &ResourceStatus) -> bool {
    let (symbol, state) = if resource.in_sync {
        ("✓".green(), "in sync")
    } else {
        ("~".yellow(), "drifted")
    };
    println!("{symbol} {} ({}, {state})", resource.name, resource.kind);
    resource.in_sync
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn is_success(code: ExitCode) -> bool {
        format!("{code:?}") == format!("{:?}", ExitCode::SUCCESS)
    }

    #[test]
    fn ignores_files_with_unmatched_conditions() {
        let directory = tempdir().unwrap();
        let condition = if cfg!(target_os = "macos") {
            "linux"
        } else {
            "macos"
        };
        fs::write(
            directory.path().join("ignored.conf"),
            format!(
                "# pets: destfile={}\n# pets: when=os:{condition}\n",
                directory.path().join("missing.conf").display()
            ),
        )
        .unwrap();

        assert!(is_success(list(directory.path().to_str().unwrap())));
    }

    #[test]
    fn reports_drift_for_files_with_matching_conditions() {
        let directory = tempdir().unwrap();
        let condition = if cfg!(target_os = "macos") {
            "macos"
        } else {
            "linux"
        };
        fs::write(
            directory.path().join("managed.conf"),
            format!(
                "# pets: destfile={}\n# pets: when=os:{condition}\n",
                directory.path().join("missing.conf").display()
            ),
        )
        .unwrap();

        assert!(!is_success(list(directory.path().to_str().unwrap())));
    }
}

use crate::pet_files;
use colored::Colorize;
use std::process::ExitCode;

pub fn list(conf_dir: &str) -> ExitCode {
    let files = match pet_files::load(conf_dir) {
        Ok(files) => files,
        Err(err) => {
            log::error!("{err}");
            return ExitCode::FAILURE;
        }
    };

    if files.is_empty() {
        println!("No pets configuration files found");
        return ExitCode::SUCCESS;
    }

    let mut all_in_sync = true;
    for pf in &files {
        let dest = pf.destination();
        let kind = if pf.is_symlink_config() {
            "symlink"
        } else {
            "destfile"
        };

        match pf.sync_status() {
            pet_files::SyncStatus::InSync => {
                println!("{} {} ({kind}, in sync)", "✓".green(), dest);
            }
            pet_files::SyncStatus::Missing | pet_files::SyncStatus::LinkMissing => {
                println!("{} {} ({kind}, missing)", "✗".red(), dest);
                all_in_sync = false;
            }
            pet_files::SyncStatus::Modified => {
                println!("{} {} ({kind}, modified)", "~".yellow(), dest);
                all_in_sync = false;
            }
            pet_files::SyncStatus::LinkWrong => {
                println!("{} {} ({kind}, wrong target)", "~".yellow(), dest);
                all_in_sync = false;
            }
        }
    }

    if all_in_sync {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

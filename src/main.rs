#![warn(clippy::pedantic)]

use clap::Parser;
use std::{env, process, time::Instant};

mod actions;
mod pet_files;
mod planner;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Pets configuration directory
    #[arg(long, default_value_t = default_conf_dir())]
    conf_dir: String,

    /// Show debugging output
    #[arg(long, default_value_t = false)]
    debug: bool,

    /// Only show changes without applying them
    #[arg(long, default_value_t = false)]
    dry_run: bool,
}

// Default configuration directory
fn default_conf_dir() -> String {
    let home_dir = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    format!("{home_dir}/pets")
}

fn setup_logging(debug: bool) {
    let level = if debug {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };
    env_logger::Builder::new().filter(None, level).init();
}

fn main() {
    let args = Args::parse();
    setup_logging(args.debug);

    let start_time = Instant::now();

    // Print distro family
    let family = actions::package_manager::which();
    match family {
        actions::PackageManager::Apt => log::debug!("Running on a Debian-like system"),
        actions::PackageManager::Yum => log::debug!("Running on a RedHat-like system"),
        actions::PackageManager::Apk => log::debug!("Running on an Alpine system"),
        actions::PackageManager::Pacman | actions::PackageManager::Yay => {
            log::debug!("Running on an Arch system");
        }
    }

    // Configuration parser
    let files = pet_files::load(&args.conf_dir).unwrap_or_else(|err| {
        log::error!("{}", err);
        vec![]
    });

    log::info!("Found {} pets configuration files", files.len());

    // Config validator
    if let Err(global_errors) = planner::check_global_constraints(&files) {
        log::error!("{}", global_errors);
        // Global validation errors mean we should stop the whole update.
        process::exit(1);
    }

    let action_plan = planner::plan_actions(files, &family);
    if args.dry_run {
        log::info!("User requested dry-run mode, not applying any changes");
    }

    // Display & Execute actions:
    // - packages to install
    // - files created/modified
    // - content diff (maybe?)
    // - owner changes
    // - permissions changes
    // - which post-update commands will be executed
    let mut exit_status = 0;
    for action in action_plan {
        if let Err(err) = action.perform(args.dry_run) {
            log::error!("Error performing action: {}", err);
            exit_status = 1;
            break;
        }
    }

    log::info!("Pets run took {:?}", start_time.elapsed());
    process::exit(exit_status);
}

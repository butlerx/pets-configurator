use clap::Parser;
use std::{env, process, time::Instant};

mod package_manager;
mod pet_files;
mod planner;
mod validator;

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
    format!("{}/pets", home_dir)
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
    let family = package_manager::which_package_manager();
    match family {
        package_manager::PackageManager::APT => log::debug!("Running on a Debian-like system"),
        package_manager::PackageManager::YUM => log::debug!("Running on a RedHat-like system"),
        package_manager::PackageManager::APK => log::debug!("Running on an Alpine system"),
        package_manager::PackageManager::PACMAN | package_manager::PackageManager::YAY => {
            log::debug!("Running on an Arch system")
        }
    }

    // Configuration parser
    let files = pet_files::load(&args.conf_dir).unwrap_or_else(|err| {
        log::error!("{}", err);
        vec![]
    });

    log::info!("Found {} pets configuration files", files.len());

    // Config validator
    if let Err(global_errors) = validator::check_global_constraints(&files) {
        log::error!("{}", global_errors);
        // Global validation errors mean we should stop the whole update.
        process::exit(1);
    }

    // Check validation errors in individual files. At this stage, the
    // command in the "pre" validation directive may not be installed yet.
    // An error in one file means we're gonna skip it but proceed with the rest.
    let good_pets = files.iter().filter(|pf| pf.is_valid()).collect::<Vec<_>>();

    /*
    // Generate the list of actions to perform.
    let actions = new_pets_actions(good_pets);

    // Display:
    // - packages to install
    // - files created/modified
    // - content diff (maybe?)
    // - owner changes
    // - permissions changes
    // - which post-update commands will be executed
    for action in &actions {
        log::info!("{}", action.command);
    }
    if args.dry_run {
        log::info!("User requested dry-run mode, not applying any changes");
        return;
    }
    */

    // Execute actions
    let exit_status = 0;
    /*
    for action in &actions {
        log::info!("Running '{}'", action.command);
        if let Err(err) = action.perform() {
            log::error!("Error performing action {}: {}", action, err);
            exit_status = 1;
            break;
        }
    }
    */

    log::info!("Pets run took {:?}", start_time.elapsed());

    process::exit(exit_status);
}

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
    #[arg(short, long, default_value_t = default_conf_dir())]
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

    // Configuration parser
    let files = pet_files::load(&args.conf_dir).unwrap_or_else(|err| {
        log::error!("{}", err);
        vec![]
    });

    log::info!("Found {} pets configuration files", files.len());
    if files.is_empty() {
        log::info!("No pets configuration files found, exiting");
        process::exit(0);
    }

    // Config validator
    if let Err(global_errors) = planner::check_global_constraints(&files) {
        log::error!("{}", global_errors);
        // Global validation errors mean we should stop the whole update.
        process::exit(1);
    }

    let action_plan = planner::plan_actions(files);
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
        match action.perform(args.dry_run) {
            Ok(status) if status != 0 => {
                exit_status = status;
                break;
            }
            Err(err) => match err {
                actions::ActionError::ExecError(cmd, status, err) => {
                    log::error!("Error: {} exited with {} => {}", cmd, status, err);
                    exit_status = status;
                    break;
                }
                actions::ActionError::IoError(err) => {
                    log::error!("Error performing action: {}", err);
                    exit_status = 1;
                    break;
                }
                _ => {
                    log::error!("Unknown error: {}", err);
                    exit_status = 1;
                    break;
                }
            },
            _ => continue,
        }
    }

    log::info!(
        "Pets run took {:.2} seconds",
        start_time.elapsed().as_secs_f64()
    );
    process::exit(exit_status);
}

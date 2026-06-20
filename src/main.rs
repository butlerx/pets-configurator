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
    #[arg(short, long, default_value_t = default_conf_dir(), env = "PETS_DIR")]
    conf_dir: String,

    /// Show debugging output
    #[arg(long, default_value_t = false)]
    debug: bool,

    /// Only show changes without applying them
    #[arg(long, default_value_t = false)]
    dry_run: bool,

    /// Check whether configuration is in sync without applying changes
    #[arg(long, default_value_t = false)]
    check: bool,
}

#[derive(Default)]
struct RunSummary {
    packages_installed: usize,
    files_created: usize,
    files_updated: usize,
    links_created: usize,
    dirs_created: usize,
    ownership_changes: usize,
    mode_changes: usize,
    post_commands: usize,
    errors: usize,
    skipped: usize,
}

impl RunSummary {
    fn increment_cause(&mut self, cause: actions::Cause) {
        match cause {
            actions::Cause::Pkg => self.packages_installed += 1,
            actions::Cause::Create => self.files_created += 1,
            actions::Cause::Update => self.files_updated += 1,
            actions::Cause::Link => self.links_created += 1,
            actions::Cause::Dir => self.dirs_created += 1,
            actions::Cause::Owner => self.ownership_changes += 1,
            actions::Cause::Mode => self.mode_changes += 1,
            actions::Cause::Post => self.post_commands += 1,
            actions::Cause::None => {}
        }
    }

    fn add_error(&mut self) {
        self.errors += 1;
    }

    fn set_skipped(&mut self, skipped: usize) {
        self.skipped = skipped;
    }

    fn log(&self) {
        let mut parts = Vec::new();
        if self.files_created > 0 {
            parts.push(format!("{} created", self.files_created));
        }
        if self.files_updated > 0 {
            parts.push(format!("{} updated", self.files_updated));
        }
        if self.links_created > 0 {
            parts.push(format!("{} links created", self.links_created));
        }
        if self.dirs_created > 0 {
            parts.push(format!("{} dirs created", self.dirs_created));
        }
        if self.packages_installed > 0 {
            parts.push(format!("{} packages installed", self.packages_installed));
        }
        if self.ownership_changes > 0 {
            parts.push(format!("{} ownership changes", self.ownership_changes));
        }
        if self.mode_changes > 0 {
            parts.push(format!("{} mode changes", self.mode_changes));
        }
        if self.post_commands > 0 {
            parts.push(format!("{} post commands", self.post_commands));
        }
        if self.skipped > 0 {
            parts.push(format!("{} already in sync", self.skipped));
        }
        if self.errors > 0 {
            parts.push(format!("{} errors", self.errors));
        }

        if parts.is_empty() {
            log::info!("Summary: no changes");
        } else {
            log::info!("Summary: {}", parts.join(", "));
        }
    }
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

    if args.check && args.dry_run {
        log::error!("--check and --dry-run are mutually exclusive");
        process::exit(1);
    }

    let start_time = Instant::now();

    // Configuration parser
    let files = pet_files::load(&args.conf_dir).unwrap_or_else(|err| {
        log::error!("{err}");
        vec![]
    });

    log::info!("Found {} pets configuration files", files.len());
    if files.is_empty() {
        log::info!("No pets configuration files found, exiting");
        process::exit(0);
    }

    // Config validator
    if let Err(global_errors) = planner::check_global_constraints(&files) {
        log::error!("{global_errors}");
        // Global validation errors mean we should stop the whole update.
        process::exit(1);
    }

    let action_plan = planner::plan_actions(files);
    let mut summary = RunSummary::default();
    if action_plan.is_empty() {
        summary.set_skipped(1);
    }

    if args.check {
        if action_plan.is_empty() {
            log::info!("Check mode: configuration is in sync");
            summary.log();
            process::exit(0);
        }

        log::warn!(
            "Check mode: drift detected ({} actions would be performed)",
            action_plan.len()
        );
        for action in &action_plan {
            log::info!("{action}");
        }
        summary.log();
        process::exit(1);
    }

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
        let cause = action.cause();
        match action.perform(args.dry_run) {
            Ok(status) if status != 0 => {
                summary.add_error();
                exit_status = status;
                break;
            }
            Ok(_) => summary.increment_cause(cause),
            Err(err) => match err {
                actions::ActionError::ExecError(cmd, status, err) => {
                    log::error!("Error: {cmd} exited with {status} => {err}");
                    summary.add_error();
                    exit_status = status;
                    break;
                }
                actions::ActionError::IoError(err) => {
                    log::error!("Error performing action: {err}");
                    summary.add_error();
                    exit_status = 1;
                    break;
                }
                _ => {
                    log::error!("Unknown error: {err}");
                    summary.add_error();
                    exit_status = 1;
                    break;
                }
            },
        }
    }

    summary.log();

    log::info!(
        "Pets run took {:.2} seconds",
        start_time.elapsed().as_secs_f64()
    );
    process::exit(exit_status);
}

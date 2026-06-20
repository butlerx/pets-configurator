#![warn(clippy::pedantic)]

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{Shell, generate};
use std::{
    env, fs, io,
    path::Path,
    process::{self, ExitCode},
    time::Instant,
};

mod actions;
mod pet_files;
mod planner;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[allow(clippy::struct_excessive_bools)]
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

    /// Disable backup creation before overwriting files
    #[arg(long, default_value_t = false)]
    no_backup: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Remove all .pets-backup files from destination directories
    CleanBackups,
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
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

fn acquire_lock() -> Option<fs::File> {
    use io::Write;

    let lock_path = env::var("PETS_LOCK_FILE").unwrap_or_else(|_| "/tmp/pets.lock".to_string());

    let file = match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock_path)
    {
        Ok(mut f) => {
            let _ = writeln!(f, "{}", process::id());
            f
        }
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            log::error!("another pets instance is running (lock file: {lock_path})");
            return None;
        }
        Err(e) => {
            log::warn!("could not create lock file {lock_path}: {e}");
            return None;
        }
    };

    Some(file)
}

fn release_lock() {
    let lock_path = env::var("PETS_LOCK_FILE").unwrap_or_else(|_| "/tmp/pets.lock".to_string());
    let _ = fs::remove_file(lock_path);
}

fn clean_backups(conf_dir: &str) -> ExitCode {
    let files = match pet_files::load(conf_dir) {
        Ok(files) => files,
        Err(err) => {
            log::error!("{err}");
            return ExitCode::FAILURE;
        }
    };

    let mut removed = 0;
    for pf in &files {
        let backup = format!("{}.pets-backup", pf.destination());
        if Path::new(&backup).exists() {
            match fs::remove_file(&backup) {
                Ok(()) => {
                    log::info!("removed {backup}");
                    removed += 1;
                }
                Err(e) => log::error!("failed to remove {backup}: {e}"),
            }
        }
    }

    if removed == 0 {
        log::info!("no backup files found");
    } else {
        log::info!("removed {removed} backup files");
    }
    ExitCode::SUCCESS
}

fn run(args: &Args) -> ExitCode {
    if args.check && args.dry_run {
        log::error!("--check and --dry-run are mutually exclusive");
        return ExitCode::FAILURE;
    }

    let _lock = if !args.dry_run && !args.check {
        match acquire_lock() {
            Some(lock) => Some(lock),
            None => return ExitCode::FAILURE,
        }
    } else {
        None
    };

    let start_time = Instant::now();

    let files = pet_files::load(&args.conf_dir).unwrap_or_else(|err| {
        log::error!("{err}");
        vec![]
    });

    log::info!("Found {} pets configuration files", files.len());
    if files.is_empty() {
        log::info!("No pets configuration files found, exiting");
        release_lock();
        return ExitCode::SUCCESS;
    }

    if let Err(global_errors) = planner::check_global_constraints(&files) {
        log::error!("{global_errors}");
        release_lock();
        return ExitCode::FAILURE;
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
            return ExitCode::SUCCESS;
        }

        log::warn!(
            "Check mode: drift detected ({} actions would be performed)",
            action_plan.len()
        );
        for action in &action_plan {
            log::info!("{action}");
        }
        summary.log();
        return ExitCode::FAILURE;
    }

    if args.dry_run {
        log::info!("User requested dry-run mode, not applying any changes");
    }

    let config = actions::RunConfig {
        dry_run: args.dry_run,
        backup: !args.no_backup,
    };

    let mut exit_code = ExitCode::SUCCESS;
    for action in action_plan {
        let cause = action.cause();
        match action.perform(&config) {
            Ok(status) if status != 0 => {
                summary.add_error();
                exit_code = ExitCode::FAILURE;
                break;
            }
            Ok(_) => summary.increment_cause(cause),
            Err(err) => {
                log::error!("{err}");
                summary.add_error();
                exit_code = ExitCode::FAILURE;
                break;
            }
        }
    }

    summary.log();

    log::info!(
        "Pets run took {:.2} seconds",
        start_time.elapsed().as_secs_f64()
    );

    release_lock();
    exit_code
}

fn main() -> ExitCode {
    let args = Args::parse();
    setup_logging(args.debug);

    match &args.command {
        Some(Commands::CleanBackups) => clean_backups(&args.conf_dir),
        Some(Commands::Completions { shell }) => {
            generate(*shell, &mut Args::command(), "pets", &mut io::stdout());
            ExitCode::SUCCESS
        }
        None => run(&args),
    }
}

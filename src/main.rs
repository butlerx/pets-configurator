#![warn(clippy::pedantic)]

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{Shell, generate};
use std::{io, process::ExitCode};

mod actions;
mod commands;
mod config_root;
mod lock;
mod manifest;
mod pet_files;
mod planner;
mod summary;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct Args {
    /// Pets configuration directory
    #[arg(short, long, default_value_t = default_conf_dir(), env = "PETS_DIR")]
    conf_dir: String,

    /// Show debugging output
    #[arg(long, default_value_t = false)]
    debug: bool,

    /// Suppress informational output, only show errors and changes
    #[arg(short, long, default_value_t = false)]
    quiet: bool,

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
    command: Option<SubCmd>,
}

#[derive(Subcommand, Debug)]
enum SubCmd {
    /// Remove all .pets-backup files from destination directories
    CleanBackups,
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
    /// Show managed files and their sync status
    #[command(alias = "status")]
    List,
    /// Synchronize the discovered or configured Pets repository
    Sync,
}

fn default_conf_dir() -> String {
    config_root::default_conf_dir()
        .to_string_lossy()
        .into_owned()
}

fn setup_logging(debug: bool, quiet: bool) {
    let level = if debug {
        log::LevelFilter::Debug
    } else if quiet {
        log::LevelFilter::Warn
    } else {
        log::LevelFilter::Info
    };
    env_logger::Builder::new().filter(None, level).init();
}

fn main() -> ExitCode {
    let args = Args::parse();
    setup_logging(args.debug, args.quiet);

    match &args.command {
        Some(SubCmd::CleanBackups) => commands::clean_backups(&args.conf_dir),
        Some(SubCmd::Completions { shell }) => {
            generate(*shell, &mut Args::command(), "pets", &mut io::stdout());
            ExitCode::SUCCESS
        }
        Some(SubCmd::List) => commands::list(&args.conf_dir),
        Some(SubCmd::Sync) | None if args.check => commands::check(&args.conf_dir),
        Some(SubCmd::Sync) | None => commands::apply(&args.conf_dir, args.dry_run, !args.no_backup),
    }
}

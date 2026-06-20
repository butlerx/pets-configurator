use crate::{actions, lock, summary::RunSummary};
use std::{process::ExitCode, time::Instant};

use super::plan::load_and_plan;

fn execute_actions(
    actions: Vec<actions::Action>,
    config: &actions::RunConfig,
) -> (ExitCode, RunSummary) {
    let mut summary = RunSummary::default();
    if actions.is_empty() {
        summary.record_skipped(1);
        return (ExitCode::SUCCESS, summary);
    }

    let mut exit_code = ExitCode::SUCCESS;
    for action in actions {
        let cause = action.cause();
        match action.perform(config) {
            Ok(0) => summary.record(cause),
            Ok(_) => {
                summary.record_error();
                exit_code = ExitCode::FAILURE;
                break;
            }
            Err(err) => {
                log::error!("{err}");
                summary.record_error();
                exit_code = ExitCode::FAILURE;
                break;
            }
        }
    }

    (exit_code, summary)
}

pub fn apply(conf_dir: &str, dry_run: bool, backup: bool) -> ExitCode {
    let _lock = if dry_run {
        None
    } else {
        match lock::Lock::acquire() {
            Ok(lock) => Some(lock),
            Err(msg) => {
                log::error!("{msg}");
                return ExitCode::FAILURE;
            }
        }
    };

    let start_time = Instant::now();

    let actions = match load_and_plan(conf_dir) {
        Ok(a) => a,
        Err(code) => return code,
    };

    if dry_run {
        log::info!("User requested dry-run mode, not applying any changes");
    }

    let config = actions::RunConfig { dry_run, backup };

    let (exit_code, summary) = execute_actions(actions, &config);
    summary.log();

    log::info!(
        "Pets run took {:.2} seconds",
        start_time.elapsed().as_secs_f64()
    );

    exit_code
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{Action, Cause, RunConfig};
    use std::fs;
    use tempfile::tempdir;

    fn is_success(code: ExitCode) -> bool {
        format!("{code:?}") == format!("{:?}", ExitCode::SUCCESS)
    }

    fn is_failure(code: ExitCode) -> bool {
        !is_success(code)
    }

    fn dry_run_config() -> RunConfig {
        RunConfig {
            dry_run: true,
            backup: false,
        }
    }

    fn real_config() -> RunConfig {
        RunConfig {
            dry_run: false,
            backup: false,
        }
    }

    #[test]
    fn execute_empty_actions_returns_success_with_skipped() {
        let (code, summary) = execute_actions(vec![], &dry_run_config());
        assert!(is_success(code));
        assert_eq!(summary.as_parts(), vec!["1 already in sync"]);
    }

    #[test]
    fn execute_copy_action_in_dry_run() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src.txt");
        let dest = dir.path().join("dest.txt");
        fs::write(&src, b"content").unwrap();

        let action = Action::copy_file(Cause::Create, src, dest);
        let (code, summary) = execute_actions(vec![action], &dry_run_config());
        assert!(is_success(code));
        assert_eq!(summary.as_parts(), vec!["1 created"]);
    }

    #[test]
    fn execute_multiple_actions_counts_correctly() {
        let dir = tempdir().unwrap();
        let src1 = dir.path().join("s1.txt");
        let dest1 = dir.path().join("d1.txt");
        let src2 = dir.path().join("s2.txt");
        let dest2 = dir.path().join("d2.txt");
        fs::write(&src1, b"a").unwrap();
        fs::write(&src2, b"b").unwrap();

        let actions = vec![
            Action::copy_file(Cause::Create, src1, dest1),
            Action::copy_file(Cause::Create, src2, dest2),
        ];
        let (code, summary) = execute_actions(actions, &dry_run_config());
        assert!(is_success(code));
        assert_eq!(summary.as_parts(), vec!["2 created"]);
    }

    #[test]
    fn execute_failing_command_records_error_and_stops() {
        let action = Action::command(Cause::Post, vec!["false".to_string()]);
        let (code, summary) = execute_actions(vec![action], &real_config());
        assert!(is_failure(code));
        assert_eq!(summary.as_parts(), vec!["1 errors"]);
    }

    #[test]
    fn execute_real_copy_creates_file() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src.txt");
        let dest = dir.path().join("dest.txt");
        fs::write(&src, b"hello").unwrap();

        let action = Action::copy_file(Cause::Create, src, dest.clone());
        let (code, _) = execute_actions(vec![action], &real_config());
        assert!(is_success(code));
        assert_eq!(fs::read_to_string(&dest).unwrap(), "hello");
    }

    #[test]
    fn apply_dry_run_does_not_create_file() {
        use std::io::Write;

        let dir = tempdir().unwrap();
        let dest = "/tmp/pets-test-apply-dryrun-nodest";
        let file = dir.path().join("test.conf");
        let mut f = fs::File::create(&file).unwrap();
        writeln!(f, "# pets: destfile={dest}").unwrap();

        let code = apply(dir.path().to_str().unwrap(), true, false);
        assert!(is_success(code));
        assert!(!std::path::Path::new(dest).exists());
    }
}

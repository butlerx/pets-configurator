use std::process::ExitCode;

use super::plan::load_and_plan;

pub fn check(conf_dir: &str) -> ExitCode {
    let actions = match load_and_plan(conf_dir) {
        Ok(a) => a,
        Err(code) => return code,
    };

    if actions.is_empty() {
        log::info!("Check mode: configuration is in sync");
        return ExitCode::SUCCESS;
    }

    log::warn!(
        "Check mode: drift detected ({} actions would be performed)",
        actions.len()
    );
    for action in &actions {
        log::info!("{action}");
    }
    ExitCode::FAILURE
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, io::Write};
    use tempfile::tempdir;

    fn is_success(code: ExitCode) -> bool {
        format!("{code:?}") == format!("{:?}", ExitCode::SUCCESS)
    }

    fn is_failure(code: ExitCode) -> bool {
        !is_success(code)
    }

    #[test]
    fn check_empty_dir_returns_success() {
        let dir = tempdir().unwrap();
        let code = check(dir.path().to_str().unwrap());
        assert!(is_success(code));
    }

    #[test]
    fn check_with_drift_returns_failure() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.conf");
        let mut f = fs::File::create(&file).unwrap();
        writeln!(f, "# pets: destfile=/tmp/pets-test-check-drift").unwrap();

        let code = check(dir.path().to_str().unwrap());
        assert!(is_failure(code));

        let _ = fs::remove_file("/tmp/pets-test-check-drift");
    }
}

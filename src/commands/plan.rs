use crate::{actions, pet_files, planner};
use std::process::ExitCode;

pub fn load_and_plan(conf_dir: &str) -> Result<Vec<actions::Action>, ExitCode> {
    let files = pet_files::load(conf_dir).map_err(|err| {
        log::error!("{err}");
        ExitCode::FAILURE
    })?;

    log::info!("Found {} pets configuration files", files.len());
    if files.is_empty() {
        log::info!("No pets configuration files found, exiting");
        return Ok(vec![]);
    }

    planner::check_global_constraints(&files).map_err(|err| {
        log::error!("{err}");
        ExitCode::FAILURE
    })?;

    Ok(planner::plan_actions(files))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, io::Write};
    use tempfile::tempdir;

    #[test]
    fn load_and_plan_empty_dir_returns_empty() {
        let dir = tempdir().unwrap();
        let actions = load_and_plan(dir.path().to_str().unwrap()).unwrap();
        assert!(actions.is_empty());
    }

    #[test]
    fn load_and_plan_nonexistent_dir_returns_empty() {
        let result = load_and_plan("/tmp/pets-definitely-does-not-exist-xyz");
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn load_and_plan_with_pets_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.conf");
        let mut f = fs::File::create(&file).unwrap();
        writeln!(f, "# pets: destfile=/tmp/pets-test-load-plan-output").unwrap();

        let actions = load_and_plan(dir.path().to_str().unwrap()).unwrap();
        assert!(!actions.is_empty());

        let _ = fs::remove_file("/tmp/pets-test-load-plan-output");
    }
}

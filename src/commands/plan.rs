use crate::{actions, manifest::Manifest, pet_files, planner};
use std::{path::Path, process::ExitCode};

pub fn load_and_plan(conf_dir: &str) -> Result<Vec<actions::Action>, ExitCode> {
    let root = Path::new(conf_dir);
    let files = pet_files::load(root).map_err(|err| {
        log::error!("{err}");
        ExitCode::FAILURE
    })?;
    let manifest_actions = Manifest::load(root)
        .and_then(|manifest| {
            manifest.map_or_else(|| Ok(Vec::new()), |manifest| manifest.plan_actions(root))
        })
        .map_err(|err| {
            log::error!("{err}");
            ExitCode::FAILURE
        })?;

    log::info!("Found {} pets configuration files", files.len());
    if files.is_empty() && manifest_actions.is_empty() {
        log::info!("No pets configuration files or manifest actions found, exiting");
        return Ok(vec![]);
    }

    planner::check_global_constraints(&files).map_err(|err| {
        log::error!("{err}");
        ExitCode::FAILURE
    })?;

    Ok(manifest_actions
        .into_iter()
        .chain(planner::plan_actions(files))
        .collect())
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
    fn load_and_plan_with_manifest_resource() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(crate::manifest::FILE_NAME),
            "version = 1\n[[generated]]\nkind = \"completion\"\nshell = \"zsh\"\ntarget = \"completions/_pets\"\n",
        )
        .unwrap();

        let actions = load_and_plan(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].cause(), crate::actions::Cause::Generate);
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

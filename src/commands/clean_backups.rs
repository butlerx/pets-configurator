use crate::{manifest::Manifest, pet_files};
use std::{
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

pub fn clean_backups(conf_dir: &str) -> ExitCode {
    let root = Path::new(conf_dir);
    let files = match pet_files::load(root) {
        Ok(files) => files,
        Err(err) => {
            log::error!("{err}");
            return ExitCode::FAILURE;
        }
    };

    let manifest = match Manifest::load(root) {
        Ok(manifest) => manifest,
        Err(err) => {
            log::error!("{err}");
            return ExitCode::FAILURE;
        }
    };
    let generated_backups = manifest
        .map(|manifest| manifest.backup_paths(root))
        .unwrap_or_default();

    let removed = files
        .iter()
        .map(|pf| PathBuf::from(format!("{}.pets-backup", pf.destination())))
        .chain(generated_backups)
        .filter(|backup| remove_backup(backup))
        .count();

    if removed == 0 {
        log::info!("no backup files found");
    } else {
        log::info!("removed {removed} backup files");
    }
    ExitCode::SUCCESS
}

fn remove_backup(backup: &Path) -> bool {
    let result = fs::symlink_metadata(backup).and_then(|metadata| {
        if metadata.is_dir() {
            fs::remove_dir_all(backup)
        } else {
            fs::remove_file(backup)
        }
    });

    match result {
        Ok(()) => {
            log::info!("removed {}", backup.display());
            true
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => {
            log::error!("failed to remove {}: {error}", backup.display());
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, path::PathBuf};
    use tempfile::tempdir;

    fn is_success(code: ExitCode) -> bool {
        format!("{code:?}") == format!("{:?}", ExitCode::SUCCESS)
    }

    #[test]
    fn clean_backups_no_backups_returns_success() {
        let dir = tempdir().unwrap();
        let code = clean_backups(dir.path().to_str().unwrap());
        assert!(is_success(code));
    }

    #[test]
    fn clean_backups_removes_backup_files() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("dest.txt");
        let backup = PathBuf::from(format!("{}.pets-backup", dest.display()));

        let file = dir.path().join("test.conf");
        let mut f = fs::File::create(&file).unwrap();
        writeln!(f, "# pets: destfile={}", dest.display()).unwrap();

        fs::write(&backup, b"old content").unwrap();
        assert!(backup.exists());

        let code = clean_backups(dir.path().to_str().unwrap());
        assert!(is_success(code));
        assert!(!backup.exists());
    }

    #[test]
    fn clean_backups_removes_backup_directories() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("config");
        let backup = PathBuf::from(format!("{}.pets-backup", dest.display()));
        fs::create_dir_all(backup.join("nested")).unwrap();
        fs::write(backup.join("nested/file"), b"old content").unwrap();
        let mut file = fs::File::create(dir.path().join("config.petsfile")).unwrap();
        writeln!(file, "# pets: symlink={}", dest.display()).unwrap();
        fs::create_dir(dir.path().join("config")).unwrap();

        let code = clean_backups(dir.path().to_str().unwrap());
        assert!(is_success(code));
        assert!(!backup.exists());
    }

    #[test]
    fn clean_backups_removes_generated_resource_backups() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("completions/_pets");
        let backup = PathBuf::from(format!("{}.pets-backup", target.display()));
        fs::create_dir_all(backup.parent().unwrap()).unwrap();
        fs::write(&backup, b"old completion").unwrap();
        fs::write(
            dir.path().join(crate::manifest::FILE_NAME),
            "version = 1\n[[generated]]\nkind = \"completion\"\nshell = \"zsh\"\ntarget = \"completions/_pets\"\n",
        )
        .unwrap();

        let code = clean_backups(dir.path().to_str().unwrap());
        assert!(is_success(code));
        assert!(!backup.exists());
    }
}

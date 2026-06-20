use crate::pet_files;
use std::{fs, path::Path, process::ExitCode};

pub fn clean_backups(conf_dir: &str) -> ExitCode {
    let files = match pet_files::load(conf_dir) {
        Ok(files) => files,
        Err(err) => {
            log::error!("{err}");
            return ExitCode::FAILURE;
        }
    };

    let removed = files
        .iter()
        .map(|pf| format!("{}.pets-backup", pf.destination()))
        .filter(|backup| Path::new(backup).exists())
        .filter(|backup| match fs::remove_file(backup) {
            Ok(()) => {
                log::info!("removed {backup}");
                true
            }
            Err(e) => {
                log::error!("failed to remove {backup}: {e}");
                false
            }
        })
        .count();

    if removed == 0 {
        log::info!("no backup files found");
    } else {
        log::info!("removed {removed} backup files");
    }
    ExitCode::SUCCESS
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
}

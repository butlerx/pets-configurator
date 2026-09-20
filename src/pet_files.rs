mod condition;
mod destination;
mod directory_walker;
pub mod mode;
mod parser;
mod pet_file;

use crate::actions::package_manager;
use directory_walker::DirectoryWalker;
pub use parser::ParseError;
pub use pet_file::{PetsFile, SyncStatus};

pub fn load<P: AsRef<std::path::Path>>(directory: P) -> Result<Vec<PetsFile>, ParseError> {
    log::debug!(
        "using configuration directory '{}'",
        directory.as_ref().display()
    );

    let pkg_manager = package_manager::which()?;
    DirectoryWalker::new(directory)
        .collect(pkg_manager)
        .map(|files| files.into_iter().filter(matches_conditions).collect())
}

fn matches_conditions(file: &PetsFile) -> bool {
    let matches = file.matches_conditions();
    if !matches {
        log::debug!(
            "skipping '{}' due to unmatched 'when' condition(s)",
            file.source()
        );
    }
    matches
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn load_ignores_files_with_unmatched_conditions() {
        let directory = tempdir().unwrap();
        let unmatched_os = if cfg!(target_os = "macos") {
            "linux"
        } else {
            "macos"
        };
        fs::write(
            directory.path().join("ignored.conf"),
            format!(
                "# pets: destfile=/tmp/pets-ignored-condition\n# pets: when=os:{unmatched_os}\n"
            ),
        )
        .unwrap();

        assert!(load(directory.path()).unwrap().is_empty());
    }
}

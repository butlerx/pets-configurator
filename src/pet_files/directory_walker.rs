use super::{ParseError, PetsFile};
use std::{
    convert::AsRef,
    path::{Path, PathBuf},
};
use walkdir::{DirEntry, WalkDir};

#[derive(Debug)]
pub struct DirectoryWalker<P: AsRef<Path>> {
    directory: P,
}

impl<P: AsRef<Path>> DirectoryWalker<P> {
    pub fn new(directory: P) -> Self {
        Self { directory }
    }

    fn into_iter(self) -> impl Iterator<Item = Result<PathBuf, ParseError>> {
        WalkDir::new(self.directory)
            .into_iter()
            .filter_entry(|e| !is_git_dir(e))
            .filter_map(std::result::Result::ok)
            .map(|entry| entry.path().to_owned())
            .filter(|path| path.is_file())
            .map(Ok)
    }

    pub fn collect(self) -> Result<Vec<PetsFile>, ParseError> {
        log::debug!(
            "using configuration directory '{}'",
            self.directory.as_ref().display()
        );

        self.into_iter()
            .filter_map(Result::ok)
            .filter_map(|path| process_pets_file(&path).transpose())
            .collect()
    }
}

fn is_git_dir(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with(".git"))
        .unwrap_or(false)
}

fn process_pets_file(path: &PathBuf) -> Result<Option<PetsFile>, ParseError> {
    match PetsFile::try_from(path) {
        Ok(pf) => Ok(Some(pf)),
        Err(error) => match error {
            ParseError::NotPetsFile => Ok(None),
            ParseError::MissingDestFile(_) => {
                log::error!("{error}");
                Ok(None)
            }
            _ => Err(error),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs::{self, File},
        io::Write,
    };
    use tempfile::TempDir;

    #[test]
    fn test_directory_walker_collects_pets_files() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Create nested directory structure
        let nested_dir = base_path.join("nested");
        fs::create_dir(&nested_dir).unwrap();

        // Create valid pets files
        let valid_pets = [
            (
                base_path.join("valid1.pets"),
                "; pets: symlink=/root/.vimrc\nsyntax on",
            ),
            (
                nested_dir.join("valid2.pets"),
                "# pets: destfile=/root/.bashrc\nalias ll='ls -alF'",
            ),
        ];

        // Create non-pets files
        let non_pets = [
            (base_path.join("invalid1.txt"), "not a pets file"),
            (nested_dir.join("invalid2.conf"), "also not a pets file"),
        ];

        // Write all files
        for (path, content) in valid_pets.iter().chain(non_pets.iter()) {
            let mut file = File::create(path).unwrap();
            writeln!(file, "{content}").unwrap();
        }

        let walker = DirectoryWalker::new(temp_dir.path());

        let result = walker.collect().unwrap();

        // Should find exactly 2 valid .pets files
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_directory_walker_handles_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let walker = DirectoryWalker::new(temp_dir.path());

        let result = walker.collect().unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_directory_walker_ignores_non_pets_files() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("not_pets.txt");

        // Create a non-pets file
        let mut file = File::create(file_path).unwrap();
        writeln!(file, "not a pets file").unwrap();

        let walker = DirectoryWalker::new(temp_dir.path());
        let result = walker.collect().unwrap();

        assert!(result.is_empty());
    }
}

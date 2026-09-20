use crate::manifest::FILE_NAME;
use std::{
    env,
    path::{Path, PathBuf},
};

pub fn default_conf_dir() -> PathBuf {
    env::current_dir()
        .ok()
        .and_then(|current| discover(&current))
        .unwrap_or_else(|| {
            env::var_os("HOME")
                .map_or_else(|| PathBuf::from("."), PathBuf::from)
                .join("pets")
        })
}

pub fn discover(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|directory| directory.join(FILE_NAME).is_file())
        .map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn discovers_manifest_in_parent_directory() {
        let root = tempdir().unwrap();
        let nested = root.path().join("one/two");
        fs::create_dir_all(&nested).unwrap();
        fs::write(root.path().join(FILE_NAME), "version = 1\n").unwrap();

        assert_eq!(discover(&nested), Some(root.path().to_path_buf()));
    }

    #[test]
    fn returns_none_without_manifest() {
        let root = tempdir().unwrap();
        assert_eq!(discover(root.path()), None);
    }
}

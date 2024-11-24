use super::parser::ParseError;
use crate::actions::{Action, Cause};
use home_dir::HomeDirExt;
use merkle_hash::{Algorithm, MerkleTree};
use std::{
    fmt, fs, io,
    path::{Path, PathBuf},
};

/// returns the sha256 of the given path.
fn sha256(path: &str) -> Result<Vec<u8>, ParseError> {
    let tree = MerkleTree::builder(path)
        .algorithm(Algorithm::Sha256)
        .build()?;
    Ok(tree.root.item.hash)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Destination {
    // Full destination path where the file has to be installed
    dest: String,
    // Directory where the file has to be installed. This is only set in
    // case we have to create the destination directory
    directory: String,
    // Is this a symbolic link or an actual file to be copied?
    link: bool,
    is_dir: bool,
}

impl fmt::Display for Destination {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.dest)
    }
}

impl From<Destination> for String {
    fn from(val: Destination) -> Self {
        val.dest
    }
}

impl Destination {
    pub fn new(dest: &String, is_symlink: bool, is_dir: bool) -> Self {
        let dest_path = match dest.expand_home() {
            Ok(path) => path,
            _ => PathBuf::from(dest),
        };
        let directory = dest_path.parent().unwrap_or_else(|| Path::new(""));
        Self {
            dest: dest_path.to_string_lossy().to_string(),
            directory: directory.to_string_lossy().to_string(),
            link: is_symlink,
            is_dir,
        }
    }

    pub fn is_symlink(&self) -> bool {
        self.link
    }

    pub fn directory(&self) -> String {
        self.directory.clone()
    }

    /// figures out if a symbolic link needs to be created, and returns the corresponding `Action`
    /// With `Cause::Link` and source as target and dest as link name needs to be created.
    pub fn needs_link(&self, source: &str) -> Option<Action> {
        if !self.link || self.dest.is_empty() {
            return None;
        }

        let source = if self.is_dir {
            let mut file = PathBuf::from(source);
            file.pop();
            file.to_string_lossy().to_string()
        } else {
            source.to_string()
        };

        match fs::symlink_metadata(&self.dest) {
            Ok(metadata) => {
                // Easy case first: Dest exists and it is not a symlink
                if !metadata.file_type().is_symlink() {
                    log::error!("{} already exists", self.dest);
                    return None;
                }

                match fs::read_link(&self.dest) {
                    Ok(path) => {
                        if source == path.to_string_lossy() {
                            // Happy path
                            log::debug!("{} is a symlink to {} already", self.dest, source);
                        } else {
                            log::error!(
                                "{} is a symlink to {} instead of {}",
                                self.dest,
                                path.display(),
                                source
                            );
                        }
                        None
                    }
                    Err(err) => {
                        log::error!("cannot read link Dest file {}: {}", self.dest, err);
                        None
                    }
                }
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                // dest does not exist yet. Happy path, we are gonna create it!
                Some(Action::new(
                    Cause::Link,
                    vec![
                        String::from("ln"),
                        String::from("-s"),
                        source,
                        self.dest.to_string(),
                    ],
                ))
            }
            Err(err) => {
                log::error!("cannot lstat Dest file {}: {}", self.dest, err);
                None
            }
        }
    }

    // returns Action with Cause Dir if there is no directory at Directory,
    // meaning that it has to be created.
    pub fn needs_dir(&self) -> Option<Action> {
        if self.directory.is_empty() {
            return None;
        }

        match fs::symlink_metadata(&self.directory) {
            Ok(metadata) => {
                // Check if the Directory is not a directory
                if !metadata.file_type().is_dir() {
                    log::error!(
                        "{} already exists and it is not a directory",
                        self.directory
                    );
                }
                log::debug!("{} already exists", self.directory);
                None
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                log::debug!("{} does not exist yet", self.directory);
                // Directory does not exist yet. Happy path, we are gonna create it!
                Some(Action::new(
                    Cause::Dir,
                    vec![
                        String::from("mkdir"),
                        String::from("-p"),
                        self.directory.clone(),
                    ],
                ))
            }
            Err(err) => {
                log::error!("cannot lstat Directory {}: {}", self.directory, err);
                None
            }
        }
    }

    /// figures out if the given trigger represents a file that needs to
    /// be updated, and returns the corresponding `Action`.
    /// Cause Update if Source needs to be copied over Dest,
    /// Create if the Destination file does not exist yet,
    /// None otherwise.
    pub fn needs_copy(&self, source: &str) -> Option<Action> {
        if self.link {
            return None;
        }

        let source = if self.is_dir {
            let mut file = PathBuf::from(source);
            file.pop();
            file.to_string_lossy().to_string()
        } else {
            source.to_string()
        };

        let command = vec![String::from("cp"), source.clone(), self.dest.to_string()];

        if !Path::new(&self.dest).exists() {
            log::debug!("{} does not exist yet", self.dest);
            return Some(Action::new(Cause::Create, command));
        }

        let sha_source = match sha256(&source) {
            Ok(hash) => hash,
            Err(err) => {
                log::error!("cannot determine sha256 of Source file {}: {}", source, err);
                return None;
            }
        };

        match sha256(&self.dest) {
            Ok(sha_dest) => {
                if sha_source == sha_dest {
                    log::debug!(
                        "same sha256 for {} and {}: {:#?}",
                        source,
                        self.dest,
                        sha_source
                    );
                    return None;
                }
                log::debug!(
                    "sha256[{}]={:#?} != sha256[{}]={:#?}",
                    source,
                    sha_source,
                    self.dest,
                    sha_dest
                );
                Some(Action::new(Cause::Update, command))
            }
            Err(err) => {
                log::error!(
                    "cannot determine sha256 of Dest file {}: {}",
                    self.dest,
                    err
                );
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::tempdir;

    #[test]
    fn test_sha256_hashing() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_file.txt");
        let content = b"hello world";
        fs::write(&file_path, content).unwrap();

        let hash_result = sha256(file_path.to_str().unwrap()).unwrap();
        let expected_hash = vec![
            185, 77, 39, 185, 147, 77, 62, 8, 165, 46, 82, 215, 218, 125, 171, 250, 196, 132, 239,
            227, 122, 83, 128, 238, 144, 136, 247, 172, 226, 239, 205, 233,
        ];

        assert_eq!(hash_result, expected_hash);
    }

    #[test]
    fn test_hashing_directory() {
        let dir = tempdir().unwrap();

        let file_path = dir.path().join("test_file.txt");
        let content = b"hello world";
        fs::write(&file_path, content).unwrap();

        let file_path = dir.path().join("test_file_2.txt");
        let content = b"foo bar";
        fs::write(&file_path, content).unwrap();

        let hash_result = sha256(dir.path().to_str().unwrap()).unwrap();
        let expected_hash = vec![
            45, 234, 137, 234, 49, 226, 240, 50, 76, 129, 183, 24, 42, 128, 162, 2, 43, 131, 207,
            219, 6, 247, 126, 228, 158, 131, 94, 24, 123, 55, 202, 79,
        ];

        assert_eq!(hash_result, expected_hash);
    }

    #[test]
    fn test_destination_from_string_conversion() {
        let dest_path = "test/path".to_string();
        let dest = Destination::new(&dest_path, false, false);

        assert_eq!(dest.dest, dest_path);
        assert_eq!(dest.directory, "test");
        assert!(!dest.link);
    }

    #[test]
    fn test_destination_needs_link_creation() {
        let dir = tempdir().unwrap();
        let dest_path = dir.path().join("link_path").to_str().unwrap().to_string();
        let source_path = dir.path().join("source_file").to_str().unwrap().to_string();

        let dest = Destination::new(&dest_path, true, false);

        // The link does not exist yet, so needs_link should return Cause::Link
        assert_eq!(
            dest.needs_link(&source_path).unwrap(),
            Action::new(
                Cause::Link,
                vec![
                    String::from("ln"),
                    String::from("-s"),
                    source_path.clone(),
                    dest_path.clone()
                ]
            )
        );
    }

    #[test]
    fn test_destination_needs_link_exists() {
        let dir = tempdir().unwrap();
        let dest_path = dir.path().join("link_path");
        let source_path = dir.path().join("source_file");

        // Create a file and a symlink pointing to it
        File::create(&source_path).unwrap();
        std::os::unix::fs::symlink(&source_path, &dest_path).unwrap();

        let dest = Destination::new(&dest_path.to_str().unwrap().to_string(), true, false);
        assert_eq!(dest.needs_link(source_path.to_str().unwrap()), None);
    }

    #[test]
    fn test_destination_needs_dir_creation() {
        let dir = tempdir().unwrap();
        let dest_path = dir.path().join("non_existing_dir/path");
        let dest = Destination::new(&dest_path.to_str().unwrap().to_string(), false, false);

        // needs_dir should return Cause::Dir because the directory does not exist
        assert_eq!(
            dest.needs_dir().unwrap(),
            Action::new(
                Cause::Dir,
                vec![
                    String::from("mkdir"),
                    String::from("-p"),
                    dest_path.parent().unwrap().to_str().unwrap().to_string()
                ]
            )
        );
    }

    #[test]
    fn test_destination_needs_dir_exists() {
        let dir = tempdir().unwrap();
        let existing_dir = dir.path().join("existing_dir");
        fs::create_dir(&existing_dir).unwrap();
        let dest = Destination::new(&existing_dir.to_str().unwrap().to_string(), false, false);

        // needs_dir should return Cause::None because the directory already exists
        assert_eq!(dest.needs_dir(), None);
    }

    #[test]
    fn test_destination_needs_copy_creation() {
        let dir = tempdir().unwrap();
        let source_file = dir.path().join("source_file.txt");
        fs::write(&source_file, b"content").unwrap();

        let dest_file = dir.path().join("dest_file.txt");
        let dest = Destination::new(&dest_file.to_str().unwrap().to_string(), false, false);

        // Destination file does not exist, so needs_copy should return Cause::Create
        assert_eq!(
            dest.needs_copy(source_file.to_str().unwrap()).unwrap(),
            Action::new(
                Cause::Create,
                vec![
                    String::from("cp"),
                    source_file.to_str().unwrap().to_string(),
                    dest_file.to_str().unwrap().to_string()
                ]
            )
        );
    }

    #[test]
    fn test_destination_needs_copy_update() {
        let dir = tempdir().unwrap();
        let source_file = dir.path().join("source_file.txt");
        let dest_file = dir.path().join("dest_file.txt");

        // Write different contents to source and destination files
        fs::write(&source_file, b"new content").unwrap();
        fs::write(&dest_file, b"old content").unwrap();

        let dest = Destination::new(&dest_file.to_str().unwrap().to_string(), false, false);

        // Destination exists and content is different, so needs_copy should return Cause::Update
        assert_eq!(
            dest.needs_copy(source_file.to_str().unwrap()).unwrap(),
            Action::new(
                Cause::Update,
                vec![
                    String::from("cp"),
                    source_file.to_str().unwrap().to_string(),
                    dest_file.to_str().unwrap().to_string()
                ]
            )
        );
    }

    #[test]
    fn test_destination_needs_copy_no_update_needed() {
        let dir = tempdir().unwrap();
        let source_file = dir.path().join("source_file.txt");
        let dest_file = dir.path().join("dest_file.txt");

        // Write identical content to source and destination files
        fs::write(&source_file, b"same content").unwrap();
        fs::write(&dest_file, b"same content").unwrap();

        let dest = Destination::new(&dest_file.to_str().unwrap().to_string(), false, false);

        // Destination exists and content is identical, so needs_copy should return Cause::None
        assert_eq!(dest.needs_copy(source_file.to_str().unwrap()), None);
    }
}

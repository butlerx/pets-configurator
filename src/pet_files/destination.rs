use crate::actions::{Action, Cause};
use sha2::{Digest, Sha256};
use std::{fmt, fs, io, path::Path};

// Sha256 returns the sha256 of the given file. Shocking, I know.
fn sha256(file_name: &str) -> Result<String, io::Error> {
    let mut file = fs::File::open(file_name)?;
    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
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
}

impl fmt::Display for Destination {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.dest)
    }
}

impl From<&String> for Destination {
    fn from(dest: &String) -> Self {
        Self {
            dest: dest.to_string(),
            directory: Path::new(dest)
                .parent()
                .unwrap_or_else(|| Path::new(""))
                .to_string_lossy()
                .to_string(),
            link: false,
        }
    }
}

impl From<Destination> for String {
    fn from(val: Destination) -> Self {
        val.dest
    }
}

impl Destination {
    pub fn link(dest: &String) -> Self {
        let mut d = Self::from(dest);
        d.link = true;
        d
    }

    pub fn directory(&self) -> String {
        self.directory.clone()
    }

    /// figures out if a symbolic link needs to be created, and returns the corresponding `Action`
    /// With Cause Link and  source as target and dest as `Link_Name` needs to be created.
    pub fn needs_link(&self, source: &str) -> Option<Action> {
        if !self.link || self.dest.is_empty() {
            return None;
        }

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
                        source.to_string(),
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
                None
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
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

        let sha_source = match sha256(source) {
            Ok(hash) => hash,
            Err(err) => {
                log::error!("cannot determine sha256 of Source file {}: {}", source, err);
                return None;
            }
        };

        let command = vec![
            String::from("cp"),
            source.to_string(),
            self.dest.to_string(),
        ];

        match sha256(&self.dest) {
            Ok(sha_dest) => {
                if sha_source == sha_dest {
                    log::debug!(
                        "same sha256 for {} and {}: {}",
                        source,
                        self.dest,
                        sha_source
                    );
                    return None;
                }
                log::debug!(
                    "sha256[{}]={} != sha256[{}]={}",
                    source,
                    sha_source,
                    self.dest,
                    sha_dest
                );
                Some(Action::new(Cause::Update, command))
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                Some(Action::new(Cause::Create, command))
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
        let expected_hash = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";

        assert_eq!(hash_result, expected_hash);
    }

    #[test]
    fn test_destination_display() {
        let dest = Destination::from(&"test/path".to_string());
        assert_eq!(format!("{dest}"), "test/path");
    }

    #[test]
    fn test_destination_from_string_conversion() {
        let dest_path = "test/path".to_string();
        let dest = Destination::from(&dest_path);

        assert_eq!(dest.dest, dest_path);
        assert_eq!(dest.directory, "test");
        assert!(!dest.link);
    }

    #[test]
    fn test_destination_needs_link_creation() {
        let dir = tempdir().unwrap();
        let dest_path = dir.path().join("link_path").to_str().unwrap().to_string();
        let source_path = dir.path().join("source_file").to_str().unwrap().to_string();

        let dest = Destination::link(&dest_path);

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

        let dest = Destination::link(&dest_path.to_str().unwrap().to_string());
        assert_eq!(dest.needs_link(source_path.to_str().unwrap()), None);
    }

    #[test]
    fn test_destination_needs_dir_creation() {
        let dir = tempdir().unwrap();
        let dest_path = dir.path().join("non_existing_dir/path");
        let dest = Destination::from(&dest_path.to_str().unwrap().to_string());

        // needs_dir should return Cause::Dir because the directory does not exist
        assert_eq!(
            dest.needs_dir().unwrap(),
            Action::new(
                Cause::Dir,
                vec![
                    String::from("mkdir"),
                    String::from("-p"),
                    dest_path.to_str().unwrap().to_string()
                ]
            )
        );
    }

    #[test]
    fn test_destination_needs_dir_exists() {
        let dir = tempdir().unwrap();
        let existing_dir = dir.path().join("existing_dir");
        fs::create_dir(&existing_dir).unwrap();
        let dest = Destination::from(&existing_dir.to_str().unwrap().to_string());

        // needs_dir should return Cause::None because the directory already exists
        assert_eq!(dest.needs_dir(), None);
    }

    #[test]
    fn test_destination_needs_copy_creation() {
        let dir = tempdir().unwrap();
        let source_file = dir.path().join("source_file.txt");
        fs::write(&source_file, b"content").unwrap();

        let dest_file = dir.path().join("dest_file.txt");
        let dest = Destination::from(&dest_file.to_str().unwrap().to_string());

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

        let dest = Destination::from(&dest_file.to_str().unwrap().to_string());

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

        let dest = Destination::from(&dest_file.to_str().unwrap().to_string());

        // Destination exists and content is identical, so needs_copy should return Cause::None
        assert_eq!(dest.needs_copy(source_file.to_str().unwrap()), None);
    }
}

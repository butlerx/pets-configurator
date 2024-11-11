use crate::planner::PetsCause;
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

    // returns PetsCause LINK if a symbolic link using source as TARGET
    // and dest as LINK_NAME needs to be created.
    pub fn needs_link(&self, source: String) -> PetsCause {
        if !self.link || self.dest.is_empty() {
            return PetsCause::None;
        }

        match fs::symlink_metadata(&self.dest) {
            Ok(metadata) => {
                // Easy case first: Dest exists and it is not a symlink
                if !metadata.file_type().is_symlink() {
                    log::error!("{} already exists", self.dest);
                    return PetsCause::None;
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
                        PetsCause::None
                    }
                    Err(err) => {
                        log::error!("cannot read link Dest file {}: {}", self.dest, err);
                        PetsCause::None
                    }
                }
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                // dest does not exist yet. Happy path, we are gonna create it!
                PetsCause::Link
            }
            Err(err) => {
                log::error!("cannot lstat Dest file {}: {}", self.dest, err);
                PetsCause::None
            }
        }
    }

    // returns PetsCause DIR if there is no directory at Directory,
    // meaning that it has to be created.
    pub fn needs_dir(&self) -> PetsCause {
        if self.directory.is_empty() {
            return PetsCause::None;
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
                PetsCause::None
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                // Directory does not exist yet. Happy path, we are gonna create it!
                PetsCause::Dir
            }
            Err(err) => {
                log::error!("cannot lstat Directory {}: {}", self.directory, err);
                PetsCause::None
            }
        }
    }

    // returns PetsCause UPDATE if Source needs to be copied over Dest,
    // CREATE if the Destination file does not exist yet, NONE otherwise.
    pub fn needs_copy(&self, source: &str) -> PetsCause {
        if self.link {
            return PetsCause::None;
        }

        let sha_source = match sha256(source) {
            Ok(hash) => hash,
            Err(err) => {
                log::error!("cannot determine sha256 of Source file {}: {}", source, err);
                return PetsCause::None;
            }
        };

        match sha256(&self.dest) {
            Ok(sha_dest) => {
                if sha_source == sha_dest {
                    log::debug!(
                        "same sha256 for {} and {}: {}",
                        source,
                        self.dest,
                        sha_source
                    );
                    return PetsCause::None;
                }
                log::debug!(
                    "sha256[{}]={} != sha256[{}]={}",
                    source,
                    sha_source,
                    self.dest,
                    sha_dest
                );
                PetsCause::Update
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => PetsCause::Create,
            Err(err) => {
                log::error!(
                    "cannot determine sha256 of Dest file {}: {}",
                    self.dest,
                    err
                );
                PetsCause::None
            }
        }
    }
}

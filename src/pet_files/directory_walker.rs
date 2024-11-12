use super::{ParseError, PetsFile};
use std::{
    convert::AsRef,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct DirectoryWalker<P: AsRef<Path>> {
    directory: P,
}

impl<P: AsRef<Path>> DirectoryWalker<P> {
    pub fn new(directory: P) -> Self {
        Self { directory }
    }

    fn into_iter(self) -> impl Iterator<Item = Result<PathBuf, ParseError>> {
        fs::read_dir(self.directory)
            .map_err(ParseError::FileError)
            .into_iter()
            .flatten()
            .filter_map(std::result::Result::ok)
            .map(|entry| entry.path())
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

fn process_pets_file(path: &PathBuf) -> Result<Option<PetsFile>, ParseError> {
    match PetsFile::try_from(path) {
        Ok(pf) => Ok(Some(pf)),
        Err(error) => match error {
            ParseError::NotPetsFile => Ok(None),
            ParseError::MissingDestFile(path) => {
                log::error!("{} is missing a desination or synlink directive", path);
                Ok(None)
            }
            _ => Err(error),
        },
    }
}

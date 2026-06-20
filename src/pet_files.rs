mod destination;
mod directory_walker;
pub mod mode;
mod parser;
mod pet_file;

use crate::actions::package_manager;
use directory_walker::DirectoryWalker;
pub use parser::ParseError;
pub use pet_file::PetsFile;

pub fn load<P: AsRef<std::path::Path>>(directory: P) -> Result<Vec<PetsFile>, ParseError> {
    log::debug!(
        "using configuration directory '{}'",
        directory.as_ref().display()
    );

    let pkg_manager = package_manager::which()?;
    DirectoryWalker::new(directory).collect(&pkg_manager)
}

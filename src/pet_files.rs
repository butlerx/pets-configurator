mod destination;
mod directory_walker;
pub mod mode;
mod parser;
mod pet_file;

use directory_walker::DirectoryWalker;
pub use parser::ParseError;
pub use pet_file::PetsFile;

pub fn load<P: AsRef<std::path::Path>>(directory: P) -> Result<Vec<PetsFile>, ParseError> {
    log::debug!(
        "using configuration directory '{}'",
        directory.as_ref().display()
    );

    DirectoryWalker::new(directory).collect()
}

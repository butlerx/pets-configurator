// Pets configuration file validator. Given a list of in-memory PetsFile(s),
// see if our sanity constraints are met. For example, we do not want multiple
// files to be installed to the same destination path. Also, all validation
// commands must succeed.

use crate::pet_files::PetsFile;
use std::collections::HashMap;

#[derive(Debug)]
pub struct DuplicateDefinitionError {
    dest: String,
    source: String,
    other: String,
}

impl DuplicateDefinitionError {
    pub fn new(dest: String, source: String, other: String) -> Self {
        Self {
            dest,
            source,
            other,
        }
    }
}

impl std::fmt::Display for DuplicateDefinitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "duplicate definition for '{}': '{}' and '{}'",
            self.dest, self.source, self.other
        )
    }
}

/// validates assumptions that must hold across all configuration files.
pub fn check_global_constraints(files: &[PetsFile]) -> Result<(), DuplicateDefinitionError> {
    let mut seen: HashMap<String, &PetsFile> = HashMap::new();

    for pf in files {
        let dest = pf.destination();
        if let Some(other) = seen.get(&dest.to_string()) {
            return Err(DuplicateDefinitionError::new(
                dest.to_string(),
                pf.source(),
                other.source(),
            ));
        }
        seen.insert(dest.to_string(), pf);
    }

    Ok(())
}

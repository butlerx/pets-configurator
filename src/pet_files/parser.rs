// Pets configuration parser. Walk through a Pets directory and parse
// modelpub use parser::ParseError;se crate::package_manager::PetsPackage;
use std::{
    collections::HashMap,
    fs::File,
    io::{self, prelude::*},
    path::Path,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("invalid pets modeline: {0}")]
    InvalidModeline(String),
    #[error("invalid keyword/argument: {0}")]
    InvalidKeyword(String),
    #[error("Error opening file: {0}")]
    FileError(#[from] io::Error),
    #[error("File not a pets file")]
    NotPetsFile,
    #[error("Neither 'destfile' nor 'symlink' directives found in '{0}'")]
    MissingDestFile(String),
    #[error("Invalid file mode: {0}")]
    InvalidFileMode(String),
}

// looks into the given file and searches for pets modelines.
// A modeline is any string which includes the 'pets:' substring.
// The line should something like:
// # pets: destfile=/etc/ssh/sshd_config, owner=root, group=root, mode=0644
// All modelines found are returned Key=Value pairs in a Vec.
pub fn read_modelines<P: AsRef<Path>>(path: P) -> Result<HashMap<String, String>, ParseError> {
    let file = File::open(path)?;
    let reader = io::BufReader::new(file);
    reader
        .lines()
        .map_while(Result::ok)
        .filter(|line| line.contains("pets:"))
        .map(extract_modeline)
        .flat_map(|result| {
            result.map(|content| {
                content
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty() && *s != "\t")
                    .map(parse_key_value)
                    .collect()
            })
        })
        .collect()
}

fn extract_modeline(line: String) -> Result<String, ParseError> {
    line.split_once("pets:")
        .map(|(_, content)| content.to_string())
        .ok_or_else(|| ParseError::InvalidModeline(line))
}

fn parse_key_value(pair: &str) -> Result<(String, String), ParseError> {
    pair.split_once('=')
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .ok_or_else(|| ParseError::InvalidKeyword(pair.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_key_value() {
        assert_eq!(
            parse_key_value("key = value").unwrap(),
            ("key".to_string(), "value".to_string())
        );
    }

    #[test]
    fn test_extract_modeline() {
        assert_eq!(
            extract_modeline("# pets: key=value".to_string()).unwrap(),
            " key=value".to_string()
        );
    }
}

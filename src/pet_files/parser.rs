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
    #[error("No package manager found on the system")]
    NoSupportedPackageManager,
    #[error("Neither 'destfile' nor 'symlink' directives found in '{0}'")]
    MissingDestFile(String),
    #[error("Invalid file mode: {0}")]
    InvalidFileMode(String),
    #[error("Error hashing source file: {0}")]
    HashError(#[from] merkle_hash::error::IndexingError),
}

// looks into the given file and searches for pets modelines.
// A modeline is any string which includes the 'pets:' substring.
// The line should something like:
// # pets: destfile=/etc/ssh/sshd_config, owner=root, group=root, mode=0644
// All modelines found are returned Key=Value pairs in a Vec.
pub fn read_modelines<P: AsRef<Path>>(path: P) -> Result<HashMap<String, Vec<String>>, ParseError> {
    log::debug!("Reading modelines from file '{:?}'", path.as_ref());
    let file = File::open(path)?;
    let reader = io::BufReader::new(file);

    let mut result = HashMap::new();
    for line in reader.lines() {
        let line = match line {
            Ok(line) if line.contains("pets:") => line,
            Ok(_) => continue,
            Err(e) => match e.kind() {
                io::ErrorKind::InvalidData => {
                    log::debug!("Invalid UTF-8 data in file, skipping file");
                    return Ok(result);
                }
                _ => return Err(e.into()),
            },
        };

        let modeline = extract_modeline(line)?;
        for r in parse_multiple_key_value(&modeline) {
            match r {
                Ok((k, v)) => {
                    result.entry(k).or_insert_with(Vec::new).push(v);
                }
                Err(e) => return Err(e),
            }
        }
    }
    Ok(result)
}

fn parse_multiple_key_value(
    content: &str,
) -> impl Iterator<Item = Result<(String, String), ParseError>> + '_ {
    content
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "\t")
        .map(parse_key_value)
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
    use std::{fs::File, io::Write};
    use tempfile::TempDir;

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

    #[test]
    fn test_parse_multiple_key_value() {
        let content = "key1=value1, key2=value2";
        let expected = vec![
            ("key1".to_string(), "value1".to_string()),
            ("key2".to_string(), "value2".to_string()),
        ];
        assert_eq!(
            parse_multiple_key_value(content)
                .collect::<Result<Vec<_>, _>>()
                .unwrap(),
            expected
        );
    }

    #[test]
    fn test_multiple_modelines() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();
        let file_path = path.join("test_file");
        let mut file = File::create(&file_path).unwrap();
        writeln!(
            file,
            "# pets: package=yay:i3lock-color\n# pets: symlink=~/.config/i3/i3lock.sh, owner=butlerx, group=butlerx, mode=0755 "
        )
        .unwrap();

        let mut expected = HashMap::new();
        expected.insert("package".to_string(), vec!["yay:i3lock-color".to_string()]);
        expected.insert(
            "symlink".to_string(),
            vec!["~/.config/i3/i3lock.sh".to_string()],
        );
        expected.insert("owner".to_string(), vec!["butlerx".to_string()]);
        expected.insert("group".to_string(), vec!["butlerx".to_string()]);
        expected.insert("mode".to_string(), vec!["0755".to_string()]);
        let actual = read_modelines(file_path).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_invalid_modeline() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();
        let file_path = path.join("test_file");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "pets: foo=bar, invalid modeline").unwrap();
        let actual = read_modelines(file_path).unwrap_err();
        assert!(matches!(actual, ParseError::InvalidKeyword(_)));
    }
}

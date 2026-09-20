use std::{
    collections::HashMap,
    fs::File,
    io::{self, prelude::*},
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("{path}:{line}: {source}")]
    Directive {
        path: PathBuf,
        line: usize,
        #[source]
        source: Box<ParseError>,
    },
    #[error("invalid keyword/argument: {0}")]
    InvalidKeyword(String),
    #[error(
        "unknown directive '{0}' (known: destfile, symlink, owner, group, mode, package, pre, post, when)"
    )]
    UnknownDirective(String),
    #[error("Error opening file: {0}")]
    FileError(#[from] io::Error),
    #[error("File not a pets file")]
    NotPetsFile,
    #[error("No package manager found on the system")]
    NoSupportedPackageManager,
    #[error("invalid package manager: {0}")]
    InvalidPackageManager(String),
    #[error("Neither 'destfile' nor 'symlink' directives found in '{0}'")]
    MissingDestFile(String),
    #[error("Invalid file mode: {0}")]
    InvalidFileMode(String),
    #[error("Invalid condition: {0}")]
    InvalidCondition(String),
    #[error("Error hashing source file: {0}")]
    HashError(#[from] merkle_hash::error::IndexingError),
}

impl ParseError {
    fn at(self, path: &Path, line: usize) -> Self {
        Self::Directive {
            path: path.to_path_buf(),
            line,
            source: Box::new(self),
        }
    }
}

// Looks for pets modelines that begin a comment line after optional whitespace.
// A modeline should look like:
// # pets: destfile=/etc/ssh/sshd_config, owner=root, group=root, mode=0644
// All modelines found are returned as key-value pairs.
const KNOWN_DIRECTIVES: &[&str] = &[
    "destfile", "symlink", "owner", "group", "mode", "package", "pre", "post", "when",
];

pub fn read_modelines<P: AsRef<Path>>(path: P) -> Result<HashMap<String, Vec<String>>, ParseError> {
    let path = path.as_ref();
    log::debug!("Reading modelines from file '{}'", path.display());
    let file = File::open(path)?;
    let reader = io::BufReader::new(file);

    reader
        .lines()
        .enumerate()
        .map_while(|(line_index, line)| match line {
            Ok(line) => Some(Ok((line_index + 1, line))),
            Err(error) if error.kind() == io::ErrorKind::InvalidData => {
                log::debug!("Invalid UTF-8 data in file, skipping file");
                None
            }
            Err(error) => Some(Err(ParseError::from(error))),
        })
        .try_fold(HashMap::<String, Vec<String>>::new(), |directives, line| {
            let (line_number, line) = line?;
            let Some(modeline) = extract_modeline(&line) else {
                return Ok(directives);
            };

            parse_multiple_key_value(modeline).try_fold(directives, |mut directives, parsed| {
                let (key, value) = parsed
                    .and_then(validate_directive)
                    .map_err(|error| error.at(path, line_number))?;
                directives.entry(key).or_default().push(value);
                Ok(directives)
            })
        })
}

fn validate_directive((key, value): (String, String)) -> Result<(String, String), ParseError> {
    KNOWN_DIRECTIVES
        .contains(&key.as_str())
        .then_some((key.clone(), value))
        .ok_or(ParseError::UnknownDirective(key))
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

fn extract_modeline(line: &str) -> Option<&str> {
    let line = line.trim_start();
    let comment = line.strip_prefix('#').or_else(|| line.strip_prefix(';'))?;
    comment.trim_start().strip_prefix("pets:")
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
        assert_eq!(extract_modeline("# pets: key=value"), Some(" key=value"));
        assert_eq!(extract_modeline("  ;pets: key=value"), Some(" key=value"));
        assert_eq!(extract_modeline("text # pets: key=value"), None);
    }

    #[test]
    fn test_inline_modeline_marker_is_ignored() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("README.md");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "Annotate your files with `# pets:` directives.").unwrap();

        let actual = read_modelines(file_path).unwrap();
        assert!(actual.is_empty());
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
    fn test_malformed_directive_reports_file_and_line() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_file");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "ordinary content").unwrap();
        writeln!(file, "# pets: invalid directive").unwrap();

        let error = read_modelines(&file_path).unwrap_err();
        assert_eq!(
            error.to_string(),
            format!(
                "{}:2: invalid keyword/argument: invalid directive",
                file_path.display()
            )
        );
    }

    #[test]
    fn test_unknown_directive() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_file");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "# pets: destfile=/etc/foo, pacakge=vim").unwrap();
        let actual = read_modelines(file_path).unwrap_err();
        assert!(matches!(
            actual,
            ParseError::Directive { source, .. }
                if matches!(*source, ParseError::UnknownDirective(_))
        ));
    }

    #[test]
    fn test_modelines_found_anywhere_in_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_file");
        let mut file = File::create(&file_path).unwrap();
        (0..100)
            .try_for_each(|i| writeln!(file, "line {i} without modelines"))
            .unwrap();
        writeln!(file, "# pets: destfile=/etc/deep-in-file").unwrap();
        let actual = read_modelines(file_path).unwrap();
        assert_eq!(actual.get("destfile").unwrap(), &vec!["/etc/deep-in-file"]);
    }

    #[test]
    fn test_modelines_at_top_with_content_below() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_file");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "# pets: destfile=/etc/foo").unwrap();
        (0..20)
            .try_for_each(|i| writeln!(file, "line {i} of content"))
            .unwrap();
        let actual = read_modelines(file_path).unwrap();
        assert_eq!(actual.get("destfile").unwrap(), &vec!["/etc/foo"]);
    }
}

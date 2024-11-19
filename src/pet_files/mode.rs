use super::parser;
use std::fmt;

#[derive(Debug, Default)]
pub struct Mode(u32);

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:o}", self.0)
    }
}

impl TryFrom<&String> for Mode {
    type Error = parser::ParseError;

    fn try_from(mode: &String) -> Result<Self, Self::Error> {
        let perm = mode.trim_start_matches('0');
        match u32::from_str_radix(perm, 8) {
            // The specified 'mode' string is valid.
            Ok(num) if num <= 0o777 => Ok(Self(num)),
            _ => Err(Self::Error::InvalidFileMode(mode.to_string())),
        }
    }
}

impl PartialEq<u32> for Mode {
    fn eq(&self, other: &u32) -> bool {
        self.0 == (other & 0o777)
    }
}

impl Mode {
    pub fn is_empty(&self) -> bool {
        self.0 == u32::MIN
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode_try_from_valid() {
        let input = "644".to_string();
        let mode = Mode::try_from(&input);
        assert!(mode.is_ok());
        assert_eq!(mode.unwrap().0, 0o644);
    }

    #[test]
    fn test_mode_try_from_invalid() {
        let input = "999".to_string();
        let mode = Mode::try_from(&input);
        assert!(mode.is_err());
        match mode.unwrap_err() {
            parser::ParseError::InvalidFileMode(m) => assert_eq!(m, "999"),
            _ => panic!("Expected InvalidFileMode error"),
        }
    }

    #[test]
    fn test_mode_is_empty() {
        let empty_mode = Mode::default();
        let non_empty_mode = Mode::try_from(&"644".to_string());

        assert!(empty_mode.is_empty());
        assert!(!non_empty_mode.unwrap().is_empty());
    }

    #[test]
    fn test_mode_leading_zero() {
        let mode = Mode::try_from(&"0644".to_string());
        assert!(mode.is_ok());
        assert_eq!(mode.unwrap().0, 0o644);
    }
}

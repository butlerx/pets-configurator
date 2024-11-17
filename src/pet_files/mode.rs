use super::parser;
use std::fmt;

#[derive(Debug, Default)]
pub struct Mode(String);

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<&String> for Mode {
    type Error = parser::ParseError;

    fn try_from(mode: &String) -> Result<Self, Self::Error> {
        let perm = mode.trim_start_matches('0');
        match u32::from_str_radix(perm, 8) {
            // The specified 'mode' string is valid.
            Ok(num) if num <= 0o777 => Ok(Self(mode.to_string())),
            _ => Err(Self::Error::InvalidFileMode(mode.to_string())),
        }
    }
}

impl Mode {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn as_u32(&self) -> Result<u32, std::num::ParseIntError> {
        u32::from_str_radix(&self.0, 8)
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
        assert_eq!(mode.unwrap().0, "644");
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
        let empty_mode = Mode(String::new());
        let non_empty_mode = Mode("644".to_string());

        assert!(empty_mode.is_empty());
        assert!(!non_empty_mode.is_empty());
    }

    #[test]
    fn test_mode_as_u32_valid() {
        let mode = Mode("644".to_string());
        let result = mode.as_u32();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0o644);
    }

    #[test]
    fn test_mode_as_u32_invalid() {
        let mode = Mode("xyz".to_string());
        let result = mode.as_u32();
        assert!(result.is_err());
    }

    #[test]
    fn test_mode_leading_zero() {
        let mode = Mode("0644".to_string());
        let result = mode.as_u32();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0o644);
    }
}

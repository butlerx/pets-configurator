use super::parser;
use std::fmt;

#[derive(Default)]
pub struct Mode(String);

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<&String> for Mode {
    type Error = parser::ParseError;

    fn try_from(mode: &String) -> Result<Self, Self::Error> {
        let mode = mode.to_string();
        match file_mode::Mode::empty().set_str(&mode) {
            // The specified 'mode' string is valid.
            Ok(()) => Ok(Self(mode)),
            Err(_) => Err(Self::Error::InvalidFileMode(mode)),
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

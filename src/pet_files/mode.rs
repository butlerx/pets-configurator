use super::parser;

pub struct Mode(String);

impl Default for Mode {
    fn default() -> Self {
        Self("".to_string())
    }
}

impl TryFrom<&String> for Mode {
    type Error = parser::ParseError;

    fn try_from(mode: &String) -> Result<Self, Self::Error> {
        let mode = mode.to_string();
        match file_mode::Mode::empty().set_str(&mode) {
            // The specified 'mode' string is valid.
            Ok(_) => Ok(Self(mode)),
            Err(_) => Err(Self::Error::InvalidFileMode(mode)),
        }
    }
}

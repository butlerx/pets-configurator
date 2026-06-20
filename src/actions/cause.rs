use colored::Colorize;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cause {
    None,
    Pkg,
    Create,
    Update,
    Link,
    Dir,
    Owner,
    Mode,
    Post,
}

impl fmt::Display for Cause {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let pets_cause = match self {
            Cause::Pkg => "PACKAGE_INSTALL".cyan().to_string(),
            Cause::Create => "FILE_CREATE".green().to_string(),
            Cause::Update => "FILE_UPDATE".yellow().to_string(),
            Cause::Link => "LINK_CREATE".green().to_string(),
            Cause::Dir => "DIR_CREATE".green().to_string(),
            Cause::Owner => "OWNER".normal().to_string(),
            Cause::Mode => "CHMOD".normal().to_string(),
            Cause::Post => "POST_UPDATE".blue().to_string(),
            Cause::None => "NONE".normal().to_string(),
        };

        write!(f, "{pets_cause}")
    }
}

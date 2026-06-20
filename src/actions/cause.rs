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

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_rendered_contains(cause: Cause, expected: &str) {
        let rendered = cause.to_string();
        assert!(
            rendered.contains(expected),
            "expected '{rendered}' to contain '{expected}'"
        );
    }

    #[test]
    fn test_display_variants() {
        assert_rendered_contains(Cause::None, "NONE");
        assert_rendered_contains(Cause::Pkg, "PACKAGE_INSTALL");
        assert_rendered_contains(Cause::Create, "FILE_CREATE");
        assert_rendered_contains(Cause::Update, "FILE_UPDATE");
        assert_rendered_contains(Cause::Link, "LINK_CREATE");
        assert_rendered_contains(Cause::Dir, "DIR_CREATE");
        assert_rendered_contains(Cause::Owner, "OWNER");
        assert_rendered_contains(Cause::Mode, "CHMOD");
        assert_rendered_contains(Cause::Post, "POST_UPDATE");
    }
}

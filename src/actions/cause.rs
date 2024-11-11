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
            Cause::Pkg => "PACKAGE_INSTALL",
            Cause::Create => "FILE_CREATE",
            Cause::Update => "FILE_UPDATE",
            Cause::Link => "LINK_CREATE",
            Cause::Dir => "DIR_CREATE",
            Cause::Owner => "OWNER",
            Cause::Mode => "CHMOD",
            Cause::Post => "POST_UPDATE",
            Cause::None => "NONE",
        };

        write!(f, "{pets_cause}")
    }
}

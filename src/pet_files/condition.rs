use super::parser::ParseError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    Hostname(String),
    Os(String),
}

impl Condition {
    pub fn parse(value: &str) -> Result<Self, ParseError> {
        let (kind, expected) = value
            .split_once(':')
            .ok_or_else(|| ParseError::InvalidCondition(value.to_string()))?;

        let kind = kind.trim().to_ascii_lowercase();
        let expected = expected.trim();
        if expected.is_empty() {
            return Err(ParseError::InvalidCondition(value.to_string()));
        }

        match kind.as_str() {
            "hostname" => Ok(Self::Hostname(expected.to_string())),
            "os" => {
                let normalized = normalize_os_name(expected)
                    .ok_or_else(|| ParseError::InvalidCondition(value.to_string()))?;
                Ok(Self::Os(normalized.to_string()))
            }
            _ => Err(ParseError::InvalidCondition(value.to_string())),
        }
    }

    pub fn is_met(&self) -> bool {
        match self {
            Self::Hostname(expected) => hostname::get()
                .ok()
                .and_then(|host| host.into_string().ok())
                .is_some_and(|host| host == *expected),
            Self::Os(expected) => {
                normalize_os_name(std::env::consts::OS).is_some_and(|actual| actual == expected)
            }
        }
    }
}

fn normalize_os_name(value: &str) -> Option<&'static str> {
    match value.to_ascii_lowercase().as_str() {
        "linux" => Some("linux"),
        "macos" | "darwin" => Some("macos"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hostname_condition() {
        assert_eq!(
            Condition::parse("hostname:myserver").unwrap(),
            Condition::Hostname("myserver".to_string())
        );
    }

    #[test]
    fn parse_os_linux_condition() {
        assert_eq!(
            Condition::parse("os:linux").unwrap(),
            Condition::Os("linux".to_string())
        );
    }

    #[test]
    fn parse_os_macos_condition() {
        assert_eq!(
            Condition::parse("os:macos").unwrap(),
            Condition::Os("macos".to_string())
        );
    }

    #[test]
    fn parse_os_darwin_alias_condition() {
        assert_eq!(
            Condition::parse("os:darwin").unwrap(),
            Condition::Os("macos".to_string())
        );
    }

    #[test]
    fn parse_invalid_condition_kind() {
        assert!(matches!(
            Condition::parse("env:prod"),
            Err(ParseError::InvalidCondition(_))
        ));
    }

    #[test]
    fn parse_invalid_condition_format() {
        assert!(matches!(
            Condition::parse("hostname"),
            Err(ParseError::InvalidCondition(_))
        ));
    }

    #[test]
    fn parse_invalid_os_condition() {
        assert!(matches!(
            Condition::parse("os:windows"),
            Err(ParseError::InvalidCondition(_))
        ));
    }

    #[test]
    fn os_condition_is_met_for_current_os() {
        let cond = match std::env::consts::OS {
            "linux" => Condition::Os("linux".to_string()),
            "macos" => Condition::Os("macos".to_string()),
            _ => return,
        };
        assert!(cond.is_met());
    }

    #[test]
    fn os_condition_is_not_met_for_other_supported_os() {
        let cond = match std::env::consts::OS {
            "linux" => Condition::Os("macos".to_string()),
            "macos" => Condition::Os("linux".to_string()),
            _ => return,
        };
        assert!(!cond.is_met());
    }

    #[test]
    fn hostname_condition_is_met_for_current_host() {
        let Ok(host_os) = hostname::get() else {
            return;
        };
        let Ok(host) = host_os.into_string() else {
            return;
        };

        let cond = Condition::Hostname(host);
        assert!(cond.is_met());
    }

    #[test]
    fn hostname_condition_is_not_met_for_other_host() {
        let cond = Condition::Hostname("definitely-not-current-hostname".to_string());
        assert!(!cond.is_met());
    }
}

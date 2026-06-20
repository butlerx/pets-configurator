use crate::pet_files::ParseError;
use std::{fmt, process::Command, str};

/// `PackageManager` available on the system.
/// Supports various package managers across Linux distributions and macOS (Homebrew).
#[derive(Clone, Copy, Debug, PartialEq, Hash, Eq)]
pub enum PackageManager {
    Apt,
    Yum,
    Apk,
    Yay,
    Pacman,
    Cargo,
    Homebrew,
}

impl fmt::Display for PackageManager {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let pkg_manager = match self {
            PackageManager::Apt => "apt",
            PackageManager::Yum => "yum",
            PackageManager::Apk => "apk",
            PackageManager::Yay => "yay",
            PackageManager::Pacman => "pacman",
            PackageManager::Cargo => "cargo",
            PackageManager::Homebrew => "homebrew",
        };
        write!(f, "{pkg_manager}")
    }
}

impl str::FromStr for PackageManager {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "apt" => Ok(PackageManager::Apt),
            "yum" => Ok(PackageManager::Yum),
            "apk" => Ok(PackageManager::Apk),
            "yay" => Ok(PackageManager::Yay),
            "pacman" => Ok(PackageManager::Pacman),
            "cargo" => Ok(PackageManager::Cargo),
            "homebrew" | "brew" => Ok(PackageManager::Homebrew),
            _ => Err("Invalid package manager".to_string()),
        }
    }
}

impl PackageManager {
    // returns the command needed to install packages on this system.
    pub fn install_command(self) -> Vec<String> {
        match self {
            PackageManager::Apt => vec![
                "apt-get".to_string(),
                "-y".to_string(),
                "install".to_string(),
            ],
            PackageManager::Yum => vec!["yum".to_string(), "-y".to_string(), "install".to_string()],
            PackageManager::Apk => vec!["apk".to_string(), "add".to_string()],
            PackageManager::Pacman => vec![
                "pacman".to_string(),
                "-S".to_string(),
                "--noconfirm".to_string(),
            ],
            PackageManager::Yay => vec![
                "yay".to_string(),
                "-S".to_string(),
                "--noconfirm".to_string(),
            ],
            PackageManager::Cargo => vec!["cargo".to_string(), "install".to_string()],
            PackageManager::Homebrew => vec!["brew".to_string(), "install".to_string()],
        }
    }

    pub fn requires_sudo(self) -> bool {
        matches!(
            self,
            PackageManager::Apt | PackageManager::Yum | PackageManager::Pacman
        )
    }
}

#[cfg(target_os = "linux")]
pub fn which() -> Result<PackageManager, ParseError> {
    let commands = [
        ("apt", vec!["--help"]),
        ("yum", vec!["--help"]),
        ("apk", vec!["--version"]),
        ("yay", vec!["--version"]),
        ("pacman", vec!["--version"]),
        ("brew", vec!["--version"]),
        ("cargo", vec!["--version"]),
    ];

    for (cmd, args) in &commands {
        let output = Command::new(*cmd).args(args).output();

        if output.is_ok() {
            match *cmd {
                "apt" => return Ok(PackageManager::Apt),
                "yum" => return Ok(PackageManager::Yum),
                "apk" => return Ok(PackageManager::Apk),
                "yay" => return Ok(PackageManager::Yay),
                "pacman" => return Ok(PackageManager::Pacman),
                "cargo" => return Ok(PackageManager::Cargo),
                "brew" => return Ok(PackageManager::Homebrew),
                _ => {}
            }
        }
    }

    Err(ParseError::NoSupportedPackageManager)
}

#[cfg(target_os = "macos")]
pub fn which() -> Result<PackageManager, ParseError> {
    let commands = [("brew", vec!["--version"]), ("cargo", vec!["--version"])];

    for (cmd, args) in &commands {
        let output = Command::new(*cmd).args(args).output();

        if output.is_ok() {
            match *cmd {
                "brew" => return Ok(PackageManager::Homebrew),
                "cargo" => return Ok(PackageManager::Cargo),
                _ => {}
            }
        }
    }

    Err(ParseError::NoSupportedPackageManager)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn which() -> Result<PackageManager, ParseError> {
    // For unsupported platforms, try to find cargo at least
    if Command::new("cargo").args(["--version"]).output().is_ok() {
        return Ok(PackageManager::Cargo);
    }
    Err(ParseError::NoSupportedPackageManager)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_display_for_each_package_manager_variant() {
        let cases = [
            (PackageManager::Apt, "apt"),
            (PackageManager::Yum, "yum"),
            (PackageManager::Apk, "apk"),
            (PackageManager::Yay, "yay"),
            (PackageManager::Pacman, "pacman"),
            (PackageManager::Cargo, "cargo"),
            (PackageManager::Homebrew, "homebrew"),
        ];

        for (manager, expected) in cases {
            assert_eq!(manager.to_string(), expected);
        }
    }

    #[test]
    fn test_from_str_valid_and_invalid_values() {
        assert_eq!(
            PackageManager::from_str("apt").unwrap(),
            PackageManager::Apt
        );
        assert_eq!(
            PackageManager::from_str("yum").unwrap(),
            PackageManager::Yum
        );
        assert_eq!(
            PackageManager::from_str("apk").unwrap(),
            PackageManager::Apk
        );
        assert_eq!(
            PackageManager::from_str("yay").unwrap(),
            PackageManager::Yay
        );
        assert_eq!(
            PackageManager::from_str("pacman").unwrap(),
            PackageManager::Pacman
        );
        assert_eq!(
            PackageManager::from_str("cargo").unwrap(),
            PackageManager::Cargo
        );
        assert_eq!(
            PackageManager::from_str("homebrew").unwrap(),
            PackageManager::Homebrew
        );
        assert_eq!(
            PackageManager::from_str("brew").unwrap(),
            PackageManager::Homebrew
        );
        assert!(PackageManager::from_str("invalid").is_err());
    }

    #[test]
    fn test_install_command_for_each_variant() {
        assert_eq!(
            PackageManager::Apt.install_command(),
            vec!["apt-get", "-y", "install"]
        );
        assert_eq!(
            PackageManager::Yum.install_command(),
            vec!["yum", "-y", "install"]
        );
        assert_eq!(PackageManager::Apk.install_command(), vec!["apk", "add"]);
        assert_eq!(
            PackageManager::Pacman.install_command(),
            vec!["pacman", "-S", "--noconfirm"]
        );
        assert_eq!(
            PackageManager::Yay.install_command(),
            vec!["yay", "-S", "--noconfirm"]
        );
        assert_eq!(
            PackageManager::Cargo.install_command(),
            vec!["cargo", "install"]
        );
        assert_eq!(
            PackageManager::Homebrew.install_command(),
            vec!["brew", "install"]
        );
    }

    #[test]
    fn test_requires_sudo() {
        assert!(PackageManager::Apt.requires_sudo());
        assert!(PackageManager::Yum.requires_sudo());
        assert!(PackageManager::Pacman.requires_sudo());
        assert!(!PackageManager::Apk.requires_sudo());
        assert!(!PackageManager::Yay.requires_sudo());
        assert!(!PackageManager::Cargo.requires_sudo());
        assert!(!PackageManager::Homebrew.requires_sudo());
    }
}

use crate::pet_files::ParseError;
use std::{fmt, process::Command, str};

/// `PackageManager` available on the system.
/// Supports various package managers across Linux distributions and macOS (Homebrew).
#[derive(Clone, Debug, PartialEq, Hash, Eq)]
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
    pub fn install_command(&self) -> Vec<String> {
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

    pub fn requires_sudo(&self) -> bool {
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
                _ => continue,
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
                _ => continue,
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

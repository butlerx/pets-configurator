use std::{fmt, process::Command, str};

/// `PackageManager` available on the system.
/// Apt on Debian-based distros, Yum on `RedHat` and derivatives.
#[derive(Clone, Debug, PartialEq, Hash, Eq)]
pub enum PackageManager {
    Apt,
    Yum,
    Apk,
    Yay,
    Pacman,
    Cargo,
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
        }
    }

    pub fn requires_sudo(&self) -> bool {
        matches!(
            self,
            PackageManager::Apt | PackageManager::Yum | PackageManager::Pacman
        )
    }
}

// Which package manager is available on the system
pub fn which() -> PackageManager {
    let commands = [
        ("apt", vec!["--help"]),
        ("yum", vec!["--help"]),
        ("apk", vec!["--version"]),
        ("yay", vec!["--version"]),
        ("pacman", vec!["--version"]),
    ];

    for (cmd, args) in &commands {
        let output = Command::new(*cmd).args(args).output();

        if output.is_ok() {
            match *cmd {
                "apt" => return PackageManager::Apt,
                "yum" => return PackageManager::Yum,
                "apk" => return PackageManager::Apk,
                "yay" => return PackageManager::Yay,
                "pacman" => return PackageManager::Pacman,
                _ => continue,
            }
        }
    }

    panic!("No package manager found on the system");
}

use std::{fmt, process::Command, str};

// A Package represents a distribution package.
#[derive(Debug, PartialEq, Clone, Hash, Eq)]
pub struct Package {
    name: String,
    package_manager: Option<PackageManager>,
}

impl Package {
    pub fn new(name: String) -> Self {
        Self {
            name,
            package_manager: None,
        }
    }
}

impl fmt::Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

// PackageManager available on the system. Apt on Debian-based distros, Yum on
// RedHat and derivatives.
#[derive(Clone, Debug, PartialEq, Hash, Eq)]
pub enum PackageManager {
    Apt,
    Yum,
    Apk,
    Yay,
    Pacman,
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
                _ => unreachable!(),
            }
        }
    }

    panic!("Unknown Package Manager");
}

impl Package {
    fn get_pkg_manager(&self) -> PackageManager {
        match &self.package_manager {
            Some(pm) => pm.clone(),
            None => which(),
        }
    }

    fn get_pkg_info(&self) -> String {
        let mut pkg_info_cmd = match self.get_pkg_manager() {
            PackageManager::Apt => {
                let mut apt_cache = Command::new("apt-cache");
                apt_cache.args(["policy", &self.name]);
                apt_cache
            }
            PackageManager::Yum => {
                let mut yum = Command::new("yum");
                yum.args(["info", &self.name]);
                yum
            }
            PackageManager::Apk => {
                let mut apk = Command::new("apk");
                apk.args(["search", "-e", &self.name]);
                apk
            }
            PackageManager::Pacman => {
                let mut pacman = Command::new("pacman");
                pacman.args(["-Si", &self.name]);
                pacman
            }
            PackageManager::Yay => {
                let mut yay = Command::new("yay");
                yay.args(["-Si", &self.name]);
                yay
            }
        };

        let output = pkg_info_cmd.output().expect("Failed to execute command");

        if !output.status.success() {
            log::error!("pkgInfoPolicy() command failed: {:?}", output);
            return String::new();
        }

        str::from_utf8(&output.stdout)
            .unwrap_or_default()
            .to_string()
    }

    // IsValid returns true if the given Package is available in the distro.
    pub fn is_valid(&self) -> bool {
        let stdout = self.get_pkg_info();

        match self.get_pkg_manager() {
            PackageManager::Apt | PackageManager::Apk if stdout.starts_with(&self.name) => {
                log::debug!("{} is a valid package name", self.name);
                true
            }
            PackageManager::Yum => {
                for line in stdout.lines() {
                    let line = line.trim();
                    if let Some(pkg_name) = line.split_once(": ") {
                        if pkg_name.0.trim() == "Name" {
                            return pkg_name.1 == self.name;
                        }
                    }
                }
                false
            }
            PackageManager::Pacman | PackageManager::Yay if !stdout.starts_with("error:") => {
                log::debug!("{} is a valid package name", self.name);
                true
            }
            _ => {
                log::error!("{} is not an available package", self.name);
                false
            }
        }
    }

    // returns true if the given Package is installed on the system.
    pub fn is_installed(&self) -> bool {
        let family = self.get_pkg_manager();
        match family {
            PackageManager::Apt => {
                let stdout = self.get_pkg_info();
                for line in stdout.lines() {
                    match line.trim().split_once(": ") {
                        Some(("Installed", version)) => return version != "(none)",
                        _ => continue,
                    }
                }

                log::error!("no 'Installed:' line in apt-cache policy {}", self.name);
                false
            }
            PackageManager::Yum => match Command::new("rpm").args(["-qa", &self.name]).status() {
                Ok(status) => status.success(),
                Err(err) => {
                    log::error!("running rpm -qa {}: {}", self.name, err);
                    false
                }
            },
            PackageManager::Apk => {
                match Command::new("apk")
                    .args(["info", "-e", &self.name])
                    .output()
                {
                    Ok(output) => {
                        self.name == str::from_utf8(&output.stdout).unwrap_or_default().trim()
                    }
                    Err(err) => {
                        log::error!("running apk info -e {}: {}", self.name, err);
                        false
                    }
                }
            }
            PackageManager::Pacman | PackageManager::Yay => {
                let package_manager = if family == PackageManager::Yay {
                    "yay"
                } else {
                    "pacman"
                };
                match Command::new(package_manager)
                    .args(["-Q", &self.name])
                    .status()
                {
                    Ok(status) => status.success(),
                    Err(err) => {
                        log::error!("running {} -Q {}: {}", package_manager, self.name, err);
                        false
                    }
                }
            }
        }
    }
}

// InstallCommand returns the command needed to install packages on this
// system.
pub fn install_command(package_manager: &PackageManager) -> Vec<String> {
    match package_manager {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkg_is_valid() {
        let pkg = Package::new("coreutils".to_string());
        assert!(pkg.is_valid());
    }

    #[test]
    fn test_pkg_is_not_valid() {
        let pkg = Package::new("obviously-this-cannot-be-valid".to_string());
        assert!(!pkg.is_valid());
    }

    #[test]
    fn test_is_installed() {
        let pkg = Package::new("binutils".to_string());
        assert!(pkg.is_installed());
    }

    #[test]
    fn test_is_not_installed() {
        let pkg = Package::new("abiword".to_string());
        assert!(!pkg.is_installed());
    }

    #[test]
    fn test_which_package_manager() {
        // This test will depend on the environment it's run in and
        // might need to be adjusted or skipped based on the actual system package manager.
        let package_manager = which();
        let valid_managers = [
            PackageManager::Apt,
            PackageManager::Yum,
            PackageManager::Apk,
            PackageManager::Yay,
            PackageManager::Pacman,
        ];
        assert!(valid_managers.contains(&package_manager));
    }

    #[test]
    fn test_install_command_for_apt() {
        assert_eq!(
            install_command(&PackageManager::Apt),
            vec![
                "apt-get".to_string(),
                "-y".to_string(),
                "install".to_string()
            ]
        );
    }

    #[test]
    fn test_install_command_for_yum() {
        assert_eq!(
            install_command(&PackageManager::Yum),
            vec!["yum".to_string(), "-y".to_string(), "install".to_string()]
        );
    }

    #[test]
    fn test_install_command_for_apk() {
        assert_eq!(
            install_command(&PackageManager::Apk),
            vec!["apk".to_string(), "add".to_string()]
        );
    }

    #[test]
    fn test_install_command_for_pacman() {
        assert_eq!(
            install_command(&PackageManager::Pacman),
            vec![
                "pacman".to_string(),
                "-S".to_string(),
                "--noconfirm".to_string()
            ]
        );
    }

    #[test]
    fn test_install_command_for_yay() {
        assert_eq!(
            install_command(&PackageManager::Yay),
            vec![
                "yay".to_string(),
                "-S".to_string(),
                "--noconfirm".to_string()
            ]
        );
    }

    #[test]
    fn test_display_trait() {
        let pkg = Package::new("test-package".to_string());
        assert_eq!(format!("{pkg}"), "test-package");
    }

    #[test]
    fn test_get_pkg_info() {
        // This test will vary based on the actual package installed.
        let pkg = Package::new("coreutils".to_string());
        let info = pkg.get_pkg_info();
        assert!(!info.is_empty(), "Package info should not be empty");
    }

    #[test]
    fn test_get_pkg_manager_with_cached_manager() {
        let mut pkg = Package::new("coreutils".to_string());
        pkg.package_manager = Some(PackageManager::Apt);
        assert_eq!(pkg.get_pkg_manager(), PackageManager::Apt);
    }

    #[test]
    fn test_get_pkg_manager_without_cached_manager() {
        let pkg = Package::new("coreutils".to_string());
        let detected_manager = pkg.get_pkg_manager();
        let valid_managers = [
            PackageManager::Apt,
            PackageManager::Yum,
            PackageManager::Apk,
            PackageManager::Yay,
            PackageManager::Pacman,
        ];
        assert!(valid_managers.contains(&detected_manager));
    }

    #[test]
    fn test_is_installed_with_non_existent_package() {
        let pkg = Package::new("non-existent-package".to_string());
        assert!(!pkg.is_installed());
    }
}

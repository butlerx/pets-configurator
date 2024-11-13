use super::package_manager::PackageManager;
use std::{fmt, process::Command, str};

// A Package represents a distribution package.
#[derive(Debug, PartialEq, Clone, Hash, Eq)]
pub struct Package {
    pub name: String,
    pub package_manager: PackageManager,
}

impl Package {
    pub fn new(name: String, default_package_manager: &PackageManager) -> Self {
        let (name, package_manager) = match name.split_once(':') {
            Some((name, manager)) => {
                let package_manager = match manager {
                    "apt" => PackageManager::Apt,
                    "yum" => PackageManager::Yum,
                    "apk" => PackageManager::Apk,
                    "yay" => PackageManager::Yay,
                    "pacman" => PackageManager::Pacman,
                    "cargo" => PackageManager::Cargo,
                    _ => default_package_manager.clone(),
                };
                (name.to_string(), package_manager)
            }
            None => (name, default_package_manager.clone()),
        };
        Self {
            name,
            package_manager,
        }
    }
}

impl fmt::Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Package {
    fn get_pkg_info(&self) -> String {
        let mut pkg_info_cmd = match self.package_manager {
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
            PackageManager::Cargo => {
                let mut cargo = Command::new("cargo");
                cargo.args(["search", "--limit=1", &self.name]);
                cargo
            }
        };

        let output = pkg_info_cmd.output().expect("Failed to execute command");

        if !output.status.success() {
            log::error!(
                "Failed to get package info: {}",
                str::from_utf8(&output.stderr).unwrap_or_default()
            );
            return String::new();
        }

        str::from_utf8(&output.stdout)
            .unwrap_or_default()
            .to_string()
    }

    // IsValid returns true if the given Package is available in the distro.
    pub fn is_valid(&self) -> bool {
        let stdout = self.get_pkg_info();

        match self.package_manager {
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
            PackageManager::Cargo if !stdout.is_empty() => match stdout.split_once(" =") {
                Some((name, _)) => {
                    log::debug!("{} is a valid package name", self.name);
                    name == self.name
                }
                None => false,
            },
            _ => {
                log::error!("{} is not an available package", self.name);
                false
            }
        }
    }

    // returns true if the given Package is installed on the system.
    pub fn is_installed(&self) -> bool {
        match self.package_manager {
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
                let package_manager = if self.package_manager == PackageManager::Yay {
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
            PackageManager::Cargo => {
                match Command::new("cargo").args(["install", "--list"]).output() {
                    Ok(output) => str::from_utf8(&output.stdout)
                        .unwrap_or_default()
                        .lines()
                        .filter(|line| !line.starts_with('\t') && !line.starts_with(' '))
                        .filter_map(|line| line.split_once(" v").map(|(name, _)| name.to_string()))
                        .collect::<Vec<String>>()
                        .contains(&self.name),
                    Err(err) => {
                        log::error!("running cargo install --list: {}", err);
                        false
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkg_is_valid() {
        let pkg = Package::new("coreutils".to_string(), &PackageManager::Apt);
        assert!(pkg.is_valid());
    }

    #[test]
    fn test_pkg_is_not_valid() {
        let pkg = Package::new(
            "obviously-this-cannot-be-valid".to_string(),
            &PackageManager::Apt,
        );
        assert!(!pkg.is_valid());
    }

    #[test]
    fn test_is_installed() {
        let pkg = Package::new("binutils".to_string(), &PackageManager::Apt);
        assert!(pkg.is_installed());
    }

    #[test]
    fn test_is_not_installed() {
        let pkg = Package::new("abiword".to_string(), &PackageManager::Apt);
        assert!(!pkg.is_installed());
    }

    #[test]
    fn test_display_trait() {
        let pkg = Package::new("test-package".to_string(), &PackageManager::Apt);
        assert_eq!(format!("{pkg}"), "test-package");
    }

    #[test]
    fn test_get_pkg_info() {
        // This test will vary based on the actual package installed.
        let pkg = Package::new("coreutils".to_string(), &PackageManager::Apt);
        let info = pkg.get_pkg_info();
        assert!(!info.is_empty(), "Package info should not be empty");
    }

    #[test]
    fn test_is_installed_with_non_existent_package() {
        let pkg = Package::new("non-existent-package".to_string(), &PackageManager::Apt);
        assert!(!pkg.is_installed());
    }
}

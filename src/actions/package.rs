use super::{package_manager::PackageManager, ActionError};
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
            Some((manager, name)) => {
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
    // returns true if the given Package is available in the distro.
    pub fn is_valid(&self) -> Result<(), ActionError> {
        log::debug!(
            "Getting package info for {} from {}",
            self.name,
            self.package_manager
        );
        let cmd_config = match self.package_manager {
            PackageManager::Apt => ("apt-cache", vec!["policy", &self.name]),
            PackageManager::Yum => ("yum", vec!["info", &self.name]),
            PackageManager::Apk => ("apk", vec!["search", "-e", &self.name]),
            PackageManager::Pacman => ("pacman", vec!["-Si", &self.name]),
            PackageManager::Yay => ("yay", vec!["-Si", &self.name]),
            PackageManager::Cargo => ("cargo", vec!["search", "--limit=1", &self.name]),
        };

        let stdout = match Command::new(cmd_config.0).args(cmd_config.1).output() {
            Ok(output) if output.status.success() => str::from_utf8(&output.stdout)
                .unwrap_or_default()
                .to_string(),
            Ok(_) => {
                return Err(ActionError::PackageNotFound(
                    self.name.clone(),
                    self.package_manager.clone(),
                ));
            }
            Err(_) => return Err(ActionError::NoPackageManager),
        };

        match self.package_manager {
            PackageManager::Apt | PackageManager::Apk if stdout.starts_with(&self.name) => {
                log::debug!("{} is a valid package name", self.name);
                Ok(())
            }
            PackageManager::Yum => {
                for line in stdout.lines() {
                    let line = line.trim();
                    if let Some(pkg_name) = line.split_once(": ") {
                        if pkg_name.0.trim() == "Name" && pkg_name.1 == self.name {
                            return Ok(());
                        }
                    }
                }
                Err(ActionError::PackageNotFound(
                    self.name.clone(),
                    self.package_manager.clone(),
                ))
            }
            PackageManager::Pacman | PackageManager::Yay if !stdout.starts_with("error:") => {
                log::debug!("{} is a valid package name", self.name);
                Ok(())
            }
            PackageManager::Cargo if !stdout.is_empty() => match stdout.split_once(" =") {
                Some((name, _)) if name == self.name => {
                    log::debug!("{} is a valid package name", self.name);
                    Ok(())
                }
                _ => Err(ActionError::PackageNotFound(
                    self.name.clone(),
                    self.package_manager.clone(),
                )),
            },
            _ => Err(ActionError::PackageNotFound(
                self.name.clone(),
                self.package_manager.clone(),
            )),
        }
    }

    // returns true if the given Package is installed on the system.
    pub fn is_installed(&self) -> Result<bool, ActionError> {
        match self.package_manager {
            PackageManager::Apt => {
                let stdout = match Command::new("apt-cache")
                    .args(["policy", &self.name])
                    .output()
                {
                    Ok(output) if output.status.success() => str::from_utf8(&output.stdout)
                        .unwrap_or_default()
                        .to_string(),
                    Ok(_) => {
                        return Err(ActionError::PackageNotFound(
                            self.name.clone(),
                            self.package_manager.clone(),
                        ));
                    }
                    Err(_) => return Err(ActionError::NoPackageManager),
                };

                for line in stdout.lines() {
                    match line.trim().split_once(": ") {
                        Some(("Installed", version)) => return Ok(version != "(none)"),
                        _ => continue,
                    }
                }

                log::error!("no 'Installed:' line in apt-cache policy {}", self.name);
                Ok(false)
            }
            PackageManager::Yum => match Command::new("rpm").args(["-qa", &self.name]).status() {
                Ok(status) => Ok(status.success()),
                Err(_) => Err(ActionError::NoPackageManager),
            },
            PackageManager::Apk => {
                match Command::new("apk")
                    .args(["info", "-e", &self.name])
                    .output()
                {
                    Ok(output) => {
                        Ok(self.name == str::from_utf8(&output.stdout).unwrap_or_default().trim())
                    }
                    Err(_) => Err(ActionError::NoPackageManager),
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
                    Ok(status) => Ok(status.success()),
                    Err(_) => Err(ActionError::NoPackageManager),
                }
            }
            PackageManager::Cargo => {
                match Command::new("cargo").args(["install", "--list"]).output() {
                    Ok(output) => {
                        let std_our = str::from_utf8(&output.stdout).unwrap_or_default();
                        let installed = parse_cargo_installed(std_our);
                        Ok(installed.contains(&self.name))
                    }
                    Err(_) => Err(ActionError::NoPackageManager),
                }
            }
        }
    }
}

fn parse_cargo_installed(output: &str) -> Vec<String> {
    output
        .lines()
        .filter(|line| !line.starts_with('\t') && !line.starts_with(' '))
        .filter_map(|line| line.split_once(" v").map(|(name, _)| name.to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::package_manager;

    #[test]
    fn test_pkg_manager_specified() {
        let family = package_manager::which();
        let pkg = Package::new("cargo:exa".to_string(), &family);
        assert_eq!(pkg.package_manager, PackageManager::Cargo);
    }

    #[test]
    fn test_pkg_is_valid() {
        let family = package_manager::which();
        let pkg = Package::new("coreutils".to_string(), &family);
        assert!(pkg.is_valid().is_ok());
    }

    #[test]
    fn test_pkg_is_not_valid() {
        let family = package_manager::which();
        let pkg = Package::new("obviously-this-cannot-be-valid".to_string(), &family);
        assert!(pkg.is_valid().is_err());
    }

    #[test]
    fn test_is_installed() {
        let family = package_manager::which();
        let pkg = Package::new("binutils".to_string(), &family);
        assert!(pkg.is_installed().unwrap());
    }

    #[test]
    fn test_is_not_installed() {
        let family = package_manager::which();
        let pkg = Package::new("abiword".to_string(), &family);
        assert!(!pkg.is_installed().unwrap());
    }

    #[test]
    fn test_is_installed_with_non_existent_package() {
        let family = package_manager::which();
        let pkg = Package::new("non-existent-package".to_string(), &family);
        assert!(!pkg.is_installed().unwrap());
    }

    #[test]
    fn test_parse_cargo_installed() {
        let output = "
alacritty v0.13.2:
    alacritty
cargo-machete v0.6.2:
    cargo-machete
cargo-workspaces v0.2.44:
    cargo-workspaces
    cargo-ws
exa v0.10.1:
    exa
";
        let installed = parse_cargo_installed(output);
        assert_eq!(
            installed,
            vec!["alacritty", "cargo-machete", "cargo-workspaces", "exa"]
        );
        assert!(installed.contains(&"exa".to_string()));
        assert!(!installed.contains(&"non-existent-package".to_string()));
    }
}

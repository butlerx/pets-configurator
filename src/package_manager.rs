use std::{env, ffi::OsStr, fmt, process::Command, str};

// A PetsPackage represents a distribution package.
#[derive(Debug, PartialEq, Clone, Hash, Eq)]
pub struct PetsPackage {
    name: String,
    package_manager: Option<PackageManager>,
}

impl PetsPackage {
    pub fn new(name: String) -> Self {
        Self {
            name,
            package_manager: None,
        }
    }
}

impl fmt::Display for PetsPackage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl AsRef<OsStr> for PetsPackage {
    fn as_ref(&self) -> &OsStr {
        self.name.as_ref()
    }
}

// PackageManager available on the system. APT on Debian-based distros, YUM on
// RedHat and derivatives.
#[derive(Clone, Debug, PartialEq, Hash, Eq)]
pub enum PackageManager {
    APT,
    YUM,
    APK,
    YAY,
    PACMAN,
}

// WhichPackageManager is available on the system
pub fn which_package_manager() -> PackageManager {
    let commands = [
        ("apt", vec!["--help"]),
        ("yum", vec!["--help"]),
        ("apk", vec!["--version"]),
        ("yay", vec!["--version"]),
        ("pacman", vec!["--version"]),
    ];

    for (cmd, args) in &commands {
        let output = Command::new(*cmd).args(args).output();

        if let Ok(_) = output {
            match *cmd {
                "apt" => return PackageManager::APT,
                "yum" => return PackageManager::YUM,
                "apk" => return PackageManager::APK,
                "yay" => return PackageManager::YAY,
                "pacman" => return PackageManager::PACMAN,
                _ => unreachable!(),
            }
        }
    }

    panic!("Unknown Package Manager");
}

impl PetsPackage {
    fn get_pkg_manager(&self) -> PackageManager {
        match &self.package_manager {
            Some(pm) => pm.clone(),
            None => which_package_manager(),
        }
    }

    fn get_pkg_info(&self) -> String {
        let mut pkg_info_cmd = match self.get_pkg_manager() {
            PackageManager::APT => {
                let mut apt_cache = Command::new("apt-cache");
                apt_cache.args(["policy", &self.name]);
                apt_cache
            }
            PackageManager::YUM => {
                let mut yum = Command::new("yum");
                yum.args(["info", &self.name]);
                yum
            }
            PackageManager::APK => {
                let mut apk = Command::new("apk");
                apk.args(["search", "-e", &self.name]);
                apk
            }
            PackageManager::PACMAN => {
                let mut pacman = Command::new("pacman");
                pacman.args(["-Si", &self.name]);
                pacman
            }
            PackageManager::YAY => {
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

    // IsValid returns true if the given PetsPackage is available in the distro.
    pub fn is_valid(&self) -> bool {
        let stdout = self.get_pkg_info();

        match self.get_pkg_manager() {
            PackageManager::APT if stdout.starts_with(&self.name) => {
                log::debug!("{} is a valid package name", self.name);
                return true;
            }
            PackageManager::YUM => {
                for line in stdout.lines() {
                    let line = line.trim();
                    if let Some(pkg_name) = line.split_once(": ") {
                        if pkg_name.0.trim() == "Name" {
                            return pkg_name.1 == self.name;
                        }
                    }
                }
            }
            PackageManager::APK if stdout.starts_with(&self.name) => {
                log::debug!("{} is a valid package name", self.name);
                return true;
            }
            PackageManager::PACMAN | PackageManager::YAY if !stdout.starts_with("error:") => {
                log::debug!("{} is a valid package name", self.name);
                return true;
            }
            _ => {}
        }

        log::error!("{} is not an available package", self.name);
        false
    }

    // returns true if the given PetsPackage is installed on the system.
    pub fn is_installed(&self) -> bool {
        let family = self.get_pkg_manager();
        match family {
            PackageManager::APT => {
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
            PackageManager::YUM => match Command::new("rpm").args(["-qa", &self.name]).status() {
                Ok(status) => status.success(),
                Err(err) => {
                    log::error!("running rpm -qa {}: {}", self.name, err);
                    false
                }
            },
            PackageManager::APK => {
                match Command::new("apk")
                    .args(["info", "-e", &self.name])
                    .output()
                {
                    Ok(output) => {
                        str::from_utf8(&output.stdout).unwrap_or_default().trim() == &self.name
                    }
                    Err(err) => {
                        log::error!("running apk info -e {}: {}", self.name, err);
                        false
                    }
                }
            }
            PackageManager::PACMAN | PackageManager::YAY => {
                let package_manager = if family == PackageManager::YAY {
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
pub fn install_command(package_manager: PackageManager) -> Command {
    match package_manager {
        PackageManager::APT => {
            let mut cmd = Command::new("apt-get");
            cmd.args(["-y", "install"])
                .envs(env::vars())
                .env("DEBIAN_FRONTEND", "noninteractive");
            cmd
        }
        PackageManager::YUM => {
            let mut cmd = Command::new("yum");
            cmd.args(["-y", "install"]);
            cmd
        }
        PackageManager::APK => {
            let mut cmd = Command::new("apk");
            cmd.args(["add"]);
            cmd
        }
        PackageManager::PACMAN => {
            let mut cmd = Command::new("pacman");
            cmd.args(["-S", "--noconfirm"]);
            cmd
        }
        PackageManager::YAY => {
            let mut cmd = Command::new("yay");
            cmd.args(["-S", "--noconfirm"]);
            cmd
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkg_is_valid() {
        let pkg = PetsPackage("coreutils".to_string());
        assert!(pkg.is_valid());
    }

    #[test]
    fn test_pkg_is_not_valid() {
        let pkg = PetsPackage("obviously-this-cannot-be valid ?".to_string());
        assert!(!pkg.is_valid());
    }

    #[test]
    fn test_is_installed() {
        let pkg = PetsPackage("binutils".to_string());
        assert!(pkg.is_installed());
    }

    #[test]
    fn test_is_not_installed() {
        let pkg = PetsPackage("abiword".to_string());
        assert!(!pkg.is_installed());

        let pkg = PetsPackage("this is getting ridiculous".to_string());
        assert!(!pkg.is_installed());
    }
}

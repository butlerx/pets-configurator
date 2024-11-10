use crate::{package_manager::PetsPackage, pet_files::PetsFile};
use std::{collections::HashSet, process::Command};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PetsCause {
    NONE,
    PKG,
    CREATE,
    UPDATE,
    LINK,
    DIR,
    OWNER,
    MODE,
    POST,
}

impl<'a> From<&'a PetsCause> for &'a str {
    fn from(pc: &'a PetsCause) -> Self {
        match pc {
            PetsCause::PKG => "PACKAGE_INSTALL",
            PetsCause::CREATE => "FILE_CREATE",
            PetsCause::UPDATE => "FILE_UPDATE",
            PetsCause::LINK => "LINK_CREATE",
            PetsCause::DIR => "DIR_CREATE",
            PetsCause::OWNER => "OWNER",
            PetsCause::MODE => "CHMOD",
            PetsCause::POST => "POST_UPDATE",
            PetsCause::NONE => "NONE",
        }
    }
}

pub struct PetsAction {
    cause: PetsCause,
    command: Command,
    trigger: Option<PetsFile>,
}

impl PetsAction {
    pub fn new(cause: PetsCause, command: Command, trigger: Option<PetsFile>) -> Self {
        PetsAction {
            cause,
            command,
            trigger,
        }
    }

    pub fn perform(&mut self) -> std::io::Result<()> {
        let output = self.command.output()?;

        if !output.stdout.is_empty() {
            log::info!(
                "stdout from Perform() -> {:?}",
                String::from_utf8_lossy(&output.stdout)
            );
        }

        if !output.stderr.is_empty() {
            log::error!(
                "stderr from Perform() -> {:?}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }
}

pub fn pkgs_to_install(triggers: &[PetsFile]) -> Vec<PetsPackage> {
    let mut pkgs = HashSet::new();

    for trigger in triggers {
        for pkg in trigger.packages() {
            if pkg.is_installed() {
                log::debug!("{} already installed", pkg);
            } else {
                log::info!("{} not installed", pkg);
                pkgs.insert(pkg.clone());
            }
        }
    }

    pkgs.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkgs_to_install() {
        let pets_files: Vec<PetsFile> = vec![];
        let (is_todo, _) = pkgs_to_install(&pets_files);
        assert_eq!(is_todo, false);

        let pf = new_test_file(
            "/dev/null",
            "binutils",
            "/etc/passwd",
            "root",
            "root",
            "0640",
            "",
            "",
        )
        .unwrap();
        let pets_files = vec![pf];
        let (is_todo, _) = pkgs_to_install(&pets_files);
        assert_eq!(is_todo, false);

        // Add another package to the mix, this time it's not installed
        pets_files[0].pkgs.push("abiword".to_string());
        let (is_todo, _) = pkgs_to_install(&pets_files);
        assert_eq!(is_todo, true);
    }

    #[test]
    fn test_file_to_copy() {
        let pf = new_test_file(
            "sample_pet/ssh/sshd_config",
            "ssh",
            "sample_pet/ssh/sshd_config",
            "root",
            "root",
            "0640",
            "",
            "",
        )
        .unwrap();
        let pa = file_to_copy(&pf);
        assert!(pa.is_none());

        let pf = new_test_file(
            "sample_pet/ssh/sshd_config",
            "ssh",
            "/tmp/polpette",
            "root",
            "root",
            "0640",
            "",
            "",
        )
        .unwrap();
        let pa = file_to_copy(&pf).unwrap();
        assert_eq!(pa.cause.to_string(), "FILE_CREATE");

        let pf = new_test_file(
            "sample_pet/ssh/sshd_config",
            "ssh",
            "sample_pet/ssh/user_ssh_config",
            "root",
            "root",
            "0640",
            "",
            "",
        )
        .unwrap();
        let pa = file_to_copy(&pf).unwrap();
        assert_eq!(pa.cause.to_string(), "FILE_UPDATE");
    }

    #[test]
    fn test_chmod() {
        let pf = new_pets_file();
        pf.source = "/dev/null".to_string();
        pf.dest = "/dev/null".to_string();

        let pa = chmod(&pf);
        assert!(pa.is_none());

        pf.add_mode("0644");
        let pa = chmod(&pf).unwrap();
        assert_eq!(pa.cause.to_string(), "CHMOD");
        assert_eq!(
            pa.command.get_program().to_str().unwrap(),
            "/bin/chmod 0644 /dev/null"
        );

        pf.dest = "/etc/passwd".to_string();
        let pa = chmod(&pf);
        assert!(pa.is_none());
    }

    #[test]
    fn test_chown() {
        let pf = new_pets_file();
        pf.source = "/dev/null".to_string();
        pf.dest = "/etc/passwd".to_string();

        let pa = chown(&pf);
        assert!(pa.is_none());

        pf.add_user("root");
        pf.add_group("root");
        let pa = chown(&pf);
        assert!(pa.is_none());

        pf.add_user("nobody");
        let pa = chown(&pf).unwrap();
        assert_eq!(pa.cause.to_string(), "OWNER");
        assert_eq!(
            pa.command.get_program().to_str().unwrap(),
            "/bin/chown nobody:root /etc/passwd"
        );
    }

    #[test]
    fn test_ln() {
        let pf = new_pets_file();
        pf.source = "sample_pet/vimrc".to_string();

        let pa = link_to_create(&pf);
        assert!(pa.is_none());

        pf.add_link("/etc/passwd".to_string());
        let pa = link_to_create(&pf);
        assert!(pa.is_none());

        pf.add_link("/tmp/vimrc".to_string());
        let pa = link_to_create(&pf).unwrap();
        assert_eq!(pa.cause.to_string(), "LINK_CREATE");
        assert_eq!(
            pa.command.get_program().to_str().unwrap(),
            "/bin/ln -s sample_pet/vimrc /tmp/vimrc"
        );
    }

    #[test]
    fn test_mkdir() {
        let pf = new_pets_file();

        let pa = dir_to_create(&pf);
        assert!(pa.is_none());

        pf.directory = "/etc".to_string();
        let pa = dir_to_create(&pf);
        assert!(pa.is_none());

        pf.directory = "/etc/polpette/al/sugo".to_string();
        let pa = dir_to_create(&pf).unwrap();
        assert_eq!(pa.cause.to_string(), "DIR_CREATE");
        assert_eq!(
            pa.command.get_program().to_str().unwrap(),
            "/bin/mkdir -p /etc/polpette/al/sugo"
        );
    }
}

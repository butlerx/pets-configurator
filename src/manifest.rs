use crate::{
    Args,
    actions::{Action, Cause},
    pet_files::ParseError,
};
use clap::{CommandFactory, ValueEnum};
use clap_complete::{Shell, generate};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

pub const FILE_NAME: &str = ".pets.toml";

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Manifest {
    version: u8,
    pub repository: Repository,
    pub generated: Vec<Generated>,
    pub package_sets: Vec<PackageSet>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Repository {
    pub submodules: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Generated {
    pub kind: String,
    pub shell: String,
    pub target: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageSet {
    pub manager: String,
    pub manifest: String,
    pub lockfile: Option<String>,
}

pub struct ResourceStatus {
    pub name: String,
    pub kind: &'static str,
    pub in_sync: bool,
}

impl Manifest {
    pub fn load(root: &Path) -> Result<Option<Self>, ParseError> {
        let path = root.join(FILE_NAME);
        if !path.is_file() {
            return Ok(None);
        }

        let content = fs::read_to_string(&path)?;
        let manifest: Self =
            toml::from_str(&content).map_err(|error| ParseError::InvalidManifest {
                path: path.clone(),
                message: error.to_string(),
            })?;

        if manifest.version != 1 {
            return Err(ParseError::InvalidManifest {
                path,
                message: format!(
                    "unsupported version {}; expected version 1",
                    manifest.version
                ),
            });
        }

        Ok(Some(manifest))
    }

    pub fn plan_actions(&self, root: &Path) -> Result<Vec<Action>, ParseError> {
        let repository = self.repository.plan(root)?;
        let package_sets = self
            .package_sets
            .iter()
            .map(|resource| resource.plan(root))
            .collect::<Result<Vec<_>, _>>()?;
        let generated = self
            .generated
            .iter()
            .map(|resource| resource.plan(root))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(repository
            .into_iter()
            .chain(package_sets.into_iter().flatten())
            .chain(generated.into_iter().flatten())
            .collect())
    }

    pub fn backup_paths(&self, root: &Path) -> Vec<PathBuf> {
        self.generated
            .iter()
            .map(|resource| {
                let target = resolve_path(root, &resource.target);
                PathBuf::from(format!("{}.pets-backup", target.to_string_lossy()))
            })
            .collect()
    }

    pub fn resource_statuses(&self, root: &Path) -> Result<Vec<ResourceStatus>, ParseError> {
        let repository = self
            .repository
            .submodules
            .then(|| {
                submodules_need_update(root).map(|needs_update| ResourceStatus {
                    name: "git submodules".to_string(),
                    kind: "repository",
                    in_sync: !needs_update,
                })
            })
            .transpose()?;
        let package_sets = self
            .package_sets
            .iter()
            .map(|resource| {
                resource.is_current(root).map(|in_sync| ResourceStatus {
                    name: resource.manifest.clone(),
                    kind: "package set",
                    in_sync,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let generated = self
            .generated
            .iter()
            .map(|resource| {
                resource.plan(root).map(|action| ResourceStatus {
                    name: resource.target.clone(),
                    kind: "generated",
                    in_sync: action.is_none(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(repository
            .into_iter()
            .chain(package_sets)
            .chain(generated)
            .collect())
    }
}

impl Repository {
    fn plan(&self, root: &Path) -> Result<Option<Action>, ParseError> {
        if !self.submodules || !submodules_need_update(root)? {
            return Ok(None);
        }

        Ok(Some(Action::command_in(
            Cause::Repository,
            ["git", "submodule", "update", "--init", "--recursive"]
                .map(str::to_string)
                .to_vec(),
            root.to_path_buf(),
        )))
    }
}

impl Generated {
    fn plan(&self, root: &Path) -> Result<Option<Action>, ParseError> {
        if self.kind != "completion" {
            return Err(invalid_manifest(
                root,
                format!("unsupported generated resource kind '{}'", self.kind),
            ));
        }

        let shell = Shell::from_str(&self.shell, true).map_err(|error| {
            invalid_manifest(
                root,
                format!("unsupported completion shell '{}': {error}", self.shell),
            )
        })?;
        let mut content = Vec::new();
        generate(shell, &mut Args::command(), "pets", &mut content);

        let target = resolve_path(root, &self.target);
        let replacing = fs::symlink_metadata(&target).is_ok();
        if replacing && fs::read(&target).is_ok_and(|current| current == content) {
            Ok(None)
        } else {
            Ok(Some(Action::generated_file(content, target, replacing)))
        }
    }
}

impl PackageSet {
    fn plan(&self, root: &Path) -> Result<Option<Action>, ParseError> {
        let state = self.state(root)?;
        if state.current {
            return Ok(None);
        }

        let install_command = if self.lockfile.is_some() {
            "ci"
        } else {
            "install"
        };
        Ok(Some(Action::stateful_command(
            Cause::Pkg,
            vec![
                "npm".to_string(),
                install_command.to_string(),
                "--prefix".to_string(),
                state.install_root.to_string_lossy().into_owned(),
                "--no-fund".to_string(),
                "--no-audit".to_string(),
            ],
            root.to_path_buf(),
            state.path,
            format!("{}\n", state.digest),
        )))
    }

    fn is_current(&self, root: &Path) -> Result<bool, ParseError> {
        Ok(self.state(root)?.current)
    }

    fn state(&self, root: &Path) -> Result<PackageSetState, ParseError> {
        if self.manager != "npm" {
            return Err(invalid_manifest(
                root,
                format!("unsupported package-set manager '{}'", self.manager),
            ));
        }

        let manifest_path = resolve_path(root, &self.manifest);
        let hasher = std::iter::once(manifest_path.clone())
            .chain(
                self.lockfile
                    .as_deref()
                    .map(|lockfile| resolve_path(root, lockfile)),
            )
            .try_fold(Sha256::new(), |mut hasher, path| -> Result<_, ParseError> {
                let content = fs::read(path)?;
                hasher.update((content.len() as u64).to_le_bytes());
                hasher.update(content);
                Ok(hasher)
            })?;
        let digest = hex_digest(&hasher.finalize());
        let install_root = manifest_path.parent().unwrap_or(root).to_path_buf();
        let state_path = install_root.join("node_modules/.pets-package-set.sha256");
        let current = fs::read_to_string(&state_path).is_ok_and(|value| value.trim() == digest);

        Ok(PackageSetState {
            current,
            digest,
            install_root,
            path: state_path,
        })
    }
}

struct PackageSetState {
    current: bool,
    digest: String,
    install_root: PathBuf,
    path: PathBuf,
}

fn submodules_need_update(root: &Path) -> Result<bool, ParseError> {
    if !root.join(".gitmodules").is_file() {
        return Err(invalid_manifest(
            root,
            "repository.submodules is true but .gitmodules is missing".to_string(),
        ));
    }

    let output = Command::new("git")
        .args(["submodule", "status", "--recursive"])
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err(invalid_manifest(
            root,
            format!(
                "git submodule status failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        ));
    }

    Ok(submodule_status_has_drift(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

fn submodule_status_has_drift(output: &str) -> bool {
    output
        .lines()
        .any(|line| matches!(line.as_bytes().first(), Some(b'-' | b'+' | b'U')))
}

fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    bytes.iter().fold(
        String::with_capacity(bytes.len() * 2),
        |mut output, byte| {
            output.push(HEX[(byte >> 4) as usize] as char);
            output.push(HEX[(byte & 0x0f) as usize] as char);
            output
        },
    )
}

fn invalid_manifest(root: &Path, message: String) -> ParseError {
    ParseError::InvalidManifest {
        path: root.join(FILE_NAME),
        message,
    }
}

pub fn resolve_path(root: &Path, value: &str) -> PathBuf {
    if value == "~" {
        return env::var_os("HOME").map_or_else(|| PathBuf::from(value), PathBuf::from);
    }
    if let Some(relative) = value.strip_prefix("~/") {
        return env::var_os("HOME")
            .map(PathBuf::from)
            .map_or_else(|| PathBuf::from(value), |home| home.join(relative));
    }

    let path = Path::new(value);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn loads_versioned_manifest() {
        let root = tempdir().unwrap();
        fs::write(
            root.path().join(FILE_NAME),
            r#"
version = 1

[repository]
submodules = true

[[generated]]
kind = "completion"
shell = "zsh"
target = "completions/_pets"

[[package_sets]]
manager = "npm"
manifest = "package.json"
lockfile = "package-lock.json"
"#,
        )
        .unwrap();

        let manifest = Manifest::load(root.path()).unwrap().unwrap();
        assert!(manifest.repository.submodules);
        assert_eq!(manifest.generated.len(), 1);
        assert_eq!(manifest.package_sets.len(), 1);
    }

    #[test]
    fn plans_and_applies_generated_completion_until_in_sync() {
        let root = tempdir().unwrap();
        fs::write(
            root.path().join(FILE_NAME),
            r#"
version = 1

[[generated]]
kind = "completion"
shell = "zsh"
target = "completions/_pets"
"#,
        )
        .unwrap();

        let manifest = Manifest::load(root.path()).unwrap().unwrap();
        let actions = manifest.plan_actions(root.path()).unwrap();
        assert_eq!(actions.len(), 1);
        actions[0]
            .clone()
            .perform(&crate::actions::RunConfig {
                dry_run: false,
                backup: true,
            })
            .unwrap();
        assert!(root.path().join("completions/_pets").is_file());
        assert!(manifest.plan_actions(root.path()).unwrap().is_empty());
        assert!(
            manifest
                .resource_statuses(root.path())
                .unwrap()
                .into_iter()
                .all(|status| status.in_sync)
        );
    }

    #[test]
    fn plans_stale_npm_package_set() {
        let root = tempdir().unwrap();
        fs::write(root.path().join("package.json"), "{}").unwrap();
        fs::write(root.path().join("package-lock.json"), "{}").unwrap();
        fs::write(
            root.path().join(FILE_NAME),
            r#"
version = 1

[[package_sets]]
manager = "npm"
manifest = "package.json"
lockfile = "package-lock.json"
"#,
        )
        .unwrap();

        let manifest = Manifest::load(root.path()).unwrap().unwrap();
        let actions = manifest.plan_actions(root.path()).unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].cause(), Cause::Pkg);
    }

    #[test]
    fn rejects_unknown_package_set_manager() {
        let root = tempdir().unwrap();
        fs::write(root.path().join("Gemfile"), "source 'https://rubygems.org'").unwrap();
        fs::write(
            root.path().join(FILE_NAME),
            "version = 1\n[[package_sets]]\nmanager = \"bundler\"\nmanifest = \"Gemfile\"\n",
        )
        .unwrap();

        let manifest = Manifest::load(root.path()).unwrap().unwrap();
        let error = manifest.plan_actions(root.path()).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("unsupported package-set manager")
        );
    }

    #[test]
    fn detects_submodule_status_drift() {
        assert!(!submodule_status_has_drift(
            " 0123456789abcdef dependency (heads/main)\n"
        ));
        assert!(submodule_status_has_drift("-0123456789abcdef dependency\n"));
        assert!(submodule_status_has_drift(
            "+0123456789abcdef dependency (heads/main)\n"
        ));
        assert!(submodule_status_has_drift("U0123456789abcdef dependency\n"));
    }

    #[test]
    fn rejects_unknown_generated_resource_kind() {
        let root = tempdir().unwrap();
        fs::write(
            root.path().join(FILE_NAME),
            "version = 1\n[[generated]]\nkind = \"template\"\nshell = \"zsh\"\ntarget = \"out\"\n",
        )
        .unwrap();

        let manifest = Manifest::load(root.path()).unwrap().unwrap();
        let error = manifest.plan_actions(root.path()).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("unsupported generated resource kind")
        );
    }

    #[test]
    fn rejects_unknown_manifest_version() {
        let root = tempdir().unwrap();
        fs::write(root.path().join(FILE_NAME), "version = 2\n").unwrap();

        let error = Manifest::load(root.path()).unwrap_err();
        assert!(error.to_string().contains("unsupported version 2"));
    }

    #[test]
    fn resolves_relative_paths_from_manifest_root() {
        let root = Path::new("/tmp/pets-root");
        assert_eq!(
            resolve_path(root, "pi/settings.json"),
            PathBuf::from("/tmp/pets-root/pi/settings.json")
        );
        assert_eq!(
            resolve_path(root, "/etc/example.conf"),
            PathBuf::from("/etc/example.conf")
        );
    }
}

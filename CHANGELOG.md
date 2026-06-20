# Changelog

## [0.5.0](https://github.com/butlerx/pets-configurator/compare/v0.4.0...v0.5.0) (2026-06-20)


### Features

* add list/status command, --quiet flag, and remove modeline limit ([459020e](https://github.com/butlerx/pets-configurator/commit/459020e8782823e518234364eb9b3f23abbf4161))
* add pip/pip3 package manager support ([08aec90](https://github.com/butlerx/pets-configurator/commit/08aec90624ce7771ff6ece3fa58f05b1be984a05))


### Code Improvements

* split commands.rs into one file per command ([048c289](https://github.com/butlerx/pets-configurator/commit/048c2890041113832e6562956033341f1a34b357))

## [0.4.0](https://github.com/butlerx/pets-configurator/compare/v0.3.3...v0.4.0) (2026-06-20)


### Features

* add atomic writes, lock file, clean-backups, and shell completions ([4c5475f](https://github.com/butlerx/pets-configurator/commit/4c5475fa0c124ee57e20aeb6c614eca4d0e88b63))
* add diff, check mode, summary, colour, conditionals, and backup ([9e16aba](https://github.com/butlerx/pets-configurator/commit/9e16aba491f4992551f22cee7244d98ce9294511))


### Bug Fixes

* correct release-please-action SHA and zizmor compliance ([5ef4b52](https://github.com/butlerx/pets-configurator/commit/5ef4b523e6d5dc18de7652003bb53a7a3d17d386))
* follow symlinks and stream package manager output ([7c1272d](https://github.com/butlerx/pets-configurator/commit/7c1272da88874dc4766a0b38e8ae8d2cd1781089))
* format docker-compose sample for yamlfmt compliance ([f00bdd5](https://github.com/butlerx/pets-configurator/commit/f00bdd54b066178c9058633d1ea0c16a5ee83c98))
* limit modeline scanning to first 50 lines and reject unknown directives ([26098db](https://github.com/butlerx/pets-configurator/commit/26098db96fb4213451f1dd43ffba6ed266bf5b4d))
* release-please should bump minor for feat commits ([83b00a5](https://github.com/butlerx/pets-configurator/commit/83b00a5bf71dc3496d77344d242c3643bd4045a9))
* remove vulnerable nix dependency via home-dir crate ([7e6fa12](https://github.com/butlerx/pets-configurator/commit/7e6fa124df3880a6621f85c2b073a20007913297))
* replace vulnerable users crate with uzers ([5d07976](https://github.com/butlerx/pets-configurator/commit/5d07976455901022349fc1648ed12c451297d004))
* resolve bugs in dry-run, post ordering, pre validation, and CI ([f73bdf3](https://github.com/butlerx/pets-configurator/commit/f73bdf308e71b578881ed9c3719092868cc4864b))


### Code Improvements

* eliminate unnecessary clones ([cc39f95](https://github.com/butlerx/pets-configurator/commit/cc39f95f496320849727f53b75cbce784fc897ac))
* replace shell-outs with native filesystem operations ([f73bdf3](https://github.com/butlerx/pets-configurator/commit/f73bdf308e71b578881ed9c3719092868cc4864b))
* split main.rs into focused modules with tests ([79daa46](https://github.com/butlerx/pets-configurator/commit/79daa46406e7c0402647c88eb04e9640f5c46ef3))


### Documentation

* add CHANGELOG.md for all previous releases ([543d9b4](https://github.com/butlerx/pets-configurator/commit/543d9b43fedb3ff9f0a19d76267ccfa5920a8a82))
* improve README with lifecycle docs, quick start, and examples ([3a8e69c](https://github.com/butlerx/pets-configurator/commit/3a8e69c86a578e33bab827d2e943101c20484146))
* rewrite samples and examples based on real usage ([d1b6664](https://github.com/butlerx/pets-configurator/commit/d1b6664af2dbec1460dff29d21e7f168de9ac52d))

## [0.3.3](https://github.com/butlerx/pets-configurator/compare/v0.3.2...v0.3.3) (2025-02-25)

### Bug Fixes

* actually check if package exists in homebrew ([d2472d2](https://github.com/butlerx/pets-configurator/commit/d2472d2))

## [0.3.2](https://github.com/butlerx/pets-configurator/compare/v0.3.1...v0.3.2) (2025-02-25)

### Code Improvements

* use compile-time targets to simplify package manager runtime logic ([f7546cd](https://github.com/butlerx/pets-configurator/commit/f7546cd))

## [0.3.1](https://github.com/butlerx/pets-configurator/compare/v0.3.0...v0.3.1) (2025-02-25)

### Bug Fixes

* correctly check if homebrew is installed ([3929552](https://github.com/butlerx/pets-configurator/commit/3929552))

## [0.3.0](https://github.com/butlerx/pets-configurator/compare/v0.2.1...v0.3.0) (2025-02-25)

### Features

* add homebrew as a package manager ([0841b62](https://github.com/butlerx/pets-configurator/commit/0841b62))
* add debug logging improvements ([16f608b](https://github.com/butlerx/pets-configurator/commit/16f608b))

## [0.2.1](https://github.com/butlerx/pets-configurator/compare/v0.2.0...v0.2.1) (2024-11-19)

### Bug Fixes

* fix parsing of file mode ([578707d](https://github.com/butlerx/pets-configurator/commit/578707d))
* specify package manager commands should be run with sudo ([8ad31fb](https://github.com/butlerx/pets-configurator/commit/8ad31fb))
* handle package managers not being installed ([137f99d](https://github.com/butlerx/pets-configurator/commit/137f99d))
* parse file modes correctly and ignore binary files ([269e33c](https://github.com/butlerx/pets-configurator/commit/269e33c))
* fix parsing of multiple arguments per line ([4ec30b8](https://github.com/butlerx/pets-configurator/commit/4ec30b8))

## [0.2.0](https://github.com/butlerx/pets-configurator/compare/v0.1.3...v0.2.0) (2024-11-16)

### Features

* add ability to link directories with .petsfile ([1315fbd](https://github.com/butlerx/pets-configurator/commit/1315fbd))

## [0.1.3](https://github.com/butlerx/pets-configurator/compare/v0.1.2...v0.1.3) (2024-11-13)

### Features

* add support for specifying the package manager per-package ([9a4ca37](https://github.com/butlerx/pets-configurator/commit/9a4ca37))

### Bug Fixes

* fix custom package install ([051f12b](https://github.com/butlerx/pets-configurator/commit/051f12b))
* add better error output ([42054b4](https://github.com/butlerx/pets-configurator/commit/42054b4))

## [0.1.2](https://github.com/butlerx/pets-configurator/compare/v0.1.1...v0.1.2) (2024-11-12)

### Bug Fixes

* correctly handle tilde (~) expansion in destination paths ([b90e125](https://github.com/butlerx/pets-configurator/commit/b90e125))

## [0.1.1](https://github.com/butlerx/pets-configurator/compare/v0.1.0...v0.1.1) (2024-11-11)

### Features

* add README documentation ([f9f9628](https://github.com/butlerx/pets-configurator/commit/f9f9628))

### Code Improvements

* refactor into modules ([9b1c99a](https://github.com/butlerx/pets-configurator/commit/9b1c99a))

## [0.1.0](https://github.com/butlerx/pets-configurator/releases/tag/v0.1.0) (2024-11-10)

### Features

* initial release
* configuration file parser with pets modeline support
* package installation via system package manager (apt, yum, apk, pacman, yay)
* file copying with SHA256 change detection
* symbolic link creation
* file ownership and permission management
* pre-update validation commands
* post-update commands

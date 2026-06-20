# Changelog

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

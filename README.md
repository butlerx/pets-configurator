# PETS Configurator

[![Crates.io](https://img.shields.io/crates/v/pets-configurator)](https://crates.io/crates/pets-configurator)
[![CI](https://github.com/butlerx/pets-configurator/actions/workflows/ci.yml/badge.svg)](https://github.com/butlerx/pets-configurator/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/crates/l/pets-configurator)](LICENSE)

A configuration management system for computers that are Pets, not Cattle.

For people who administer a handful of machines — a laptop, a workstation, that
personal server in Sweden — all fairly different and all Very Important. These
systems are not Cattle. They're Pets. Almost Family.

This is a Rust rewrite of the original [Pets](https://github.com/ema/pets)
project. See the original for
[design decisions](https://github.com/ema/pets/tree/master?tab=readme-ov-file#design-overview).

## How it works

Pets is driven by comments embedded in config files, not a DSL. Drop config
files into a directory (`~/pets` by default), annotate them with `# pets:`
directives, and run `pets`. It will:

1. **Install packages** listed in `package` directives
2. **Validate** changes with the `pre` command (if specified)
3. **Copy or symlink** files to their destination
4. **Set ownership and permissions** (`owner`, `group`, `mode`)
5. **Run post-update commands** like service reloads (`post`)

Files without `# pets:` directives are ignored. Directory structure is
arbitrary.

## Quick start

```bash
cargo install pets-configurator
```

Create `~/pets/gitconfig` with:

```gitconfig
# pets: package=git
# pets: symlink=~/.gitconfig

[user]
    name = Your Name
    email = you@example.com
[push]
    default = simple
```

Preview what would happen:

```bash
pets --dry-run
```

Apply:

```bash
pets
```

## Usage

```
pets [OPTIONS] [COMMAND]

Options:
    --conf-dir <DIR>  Configuration directory [default: ~/pets, env: PETS_DIR]
    --check           Check for drift without applying changes (exit 1 if drift)
    --debug           Show debugging output
    --dry-run         Show changes with diffs without applying them
    --no-backup       Disable backup creation before overwriting files
-h, --help            Print help
-V, --version         Print version

Commands:
    clean-backups     Remove all .pets-backup files from destination directories
    completions       Generate shell completions (bash, zsh, fish, etc.)
```

To use a different configuration directory:

```bash
pets --conf-dir /etc/pets
# or
export PETS_DIR=/etc/pets
pets
```

Preview changes with diffs:

```bash
sudo pets --dry-run
```

Check for drift in CI or cron (exits non-zero if anything is out of sync):

```bash
sudo pets --check
```

Clean up backup files:

```bash
sudo pets clean-backups
```

Generate shell completions:

```bash
pets completions bash > /etc/bash_completion.d/pets
pets completions zsh > ~/.zfunc/_pets
pets completions fish > ~/.config/fish/completions/pets.fish
```

See [sample_pet](./sample_pet) for example configurations.

## Supported platforms

| Platform | Package managers |
| --- | --- |
| Debian / Ubuntu | apt |
| RHEL / Fedora | yum |
| Alpine | apk |
| Arch Linux | pacman, yay |
| macOS | Homebrew |
| Cross-platform | Cargo |

## Configuration directives

Directives are embedded as comments in your config files using `# pets:` (or
`; pets:` for ini-style files). They can be on a single line separated by
commas, or on multiple lines:

```
# pets: destfile=/etc/ssh/sshd_config, owner=root, group=root, mode=0644
# pets: package=ssh
# pets: pre=/usr/sbin/sshd -t -f
# pets: post=/bin/systemctl reload ssh.service
```

### Available directives

| Directive | Description |
| --- | --- |
| `destfile` | Destination path to copy the file to. Required unless `symlink` is used. |
| `symlink` | Create a symbolic link at this path instead of copying. |
| `owner` | File owner (e.g. `root`). |
| `group` | File group (e.g. `staff`). |
| `mode` | Octal file permissions (e.g. `0644`). |
| `package` | Package to install before deploying. Can be specified multiple times. Prefix with a package manager to override the default: `cargo:exa`, `yay:i3lock-color`. |
| `pre` | Validation command. Must exit 0 for the file to be deployed. The source file path is appended as an argument. |
| `post` | Command to run after the file is deployed (e.g. restart a service). |
| `when` | Conditional directive. File is only applied when all conditions match. Supports `hostname:<name>` and `os:linux` / `os:macos`. Can be specified multiple times (AND logic). |

### Directory symlinks

To symlink an entire directory, create a `.petsfile` inside it with a `symlink`
directive:

```
# pets: symlink=~/.config/i3
```

The parent directory of the `.petsfile` will be symlinked to the target.

### Conditional deployment

Use `when` directives to apply files only on specific hosts or operating
systems. All conditions must match (AND logic). Files without `when` directives
are always applied.

```
# pets: destfile=/etc/apt/sources.list.d/custom.list
# pets: when=os:linux
# pets: when=hostname:webserver

deb http://example.com/repo stable main
```

Supported conditions:

| Condition | Example | Matches when |
| --- | --- | --- |
| `hostname:<name>` | `when=hostname:myserver` | System hostname matches exactly |
| `os:linux` | `when=os:linux` | Running on Linux |
| `os:macos` | `when=os:macos` | Running on macOS (also accepts `os:darwin`) |

### Backups

When updating an existing file, pets automatically creates a backup at
`<destfile>.pets-backup` before overwriting. This only happens on real runs, not
during `--dry-run` or `--check`.

Use `--no-backup` to disable this behaviour. Use `pets clean-backups` to remove
all existing backup files.

## Examples

The most common use case is managing dotfiles across machines. Store your config
files in a git repo, add `# pets:` directives, and run `pets` to symlink or
copy them into place.

See [sample_pet](./sample_pet) for a complete example.

### Shell configuration (symlink with packages)

```zsh
# pets: package=zsh
# pets: package=cargo:exa
# pets: package=bat
# pets: symlink=~/.zshrc

fpath=(~/.zsh-completions $fpath)

alias ll='exa -la --git'
alias cat='bat --paging=never'
```

### Git configuration (mixed package managers)

```gitconfig
# pets: package=git, package=apt:gh, package=yay:github-cli
# pets: symlink=~/.gitconfig

[user]
    name = Your Name
    email = you@example.com
[push]
    default = simple
[pull]
    rebase = true
```

### Terminal emulator (platform-specific build deps)

```toml
# pets: package=cargo:alacritty
# pets: package=apt:cmake, package=apt:pkg-config, package=apt:libfreetype6-dev
# pets: symlink=~/.config/alacritty/alacritty.toml

[font]
size = 9.0

[window]
opacity = 0.95
```

### Directory symlink (neovim config)

Create a `.petsfile` inside the directory:

```
# pets: symlink=~/.config/nvim
# pets: package=ripgrep, package=perl
```

The entire directory is symlinked to `~/.config/nvim`.

### SSH server (destfile with validation)

```sshd
# pets: destfile=/etc/ssh/sshd_config, owner=root, group=root, mode=0644
# pets: package=ssh
# pets: pre=/usr/sbin/sshd -t -f
# pets: post=/bin/systemctl reload ssh.service
# pets: when=os:linux

PasswordAuthentication no
PubkeyAuthentication yes
PermitRootLogin no
```

### Linux-only window manager

```i3
# pets: package=yay:i3-wm, package=apt:i3
# pets: package=rofi, package=playerctl
# pets: symlink=~/.config/i3/config
# pets: when=os:linux

set $mod Mod4
bindsym $mod+Return exec alacritty
bindsym $mod+d exec rofi -show drun
```

### Docker compose deployment

```yaml
# pets: destfile=/opt/myapp/docker-compose.yml, owner=root, group=docker, mode=0640
# pets: package=docker.io
# pets: package=docker-compose
# pets: post=/usr/bin/docker-compose -f /opt/myapp/docker-compose.yml up -d
# pets: when=os:linux

services:
  web:
    image: nginx:alpine
    ports:
      - "80:80"
  cache:
    image: redis:alpine
```

### Systemd timer (scheduled backups)

Service unit:

```ini
# pets: destfile=/etc/systemd/system/backup.service, owner=root, group=root, mode=0644
# pets: post=/bin/systemctl daemon-reload
# pets: when=os:linux

[Unit]
Description=System backup service

[Service]
Type=oneshot
ExecStart=/usr/local/bin/backup.sh
```

Timer unit:

```ini
# pets: destfile=/etc/systemd/system/backup.timer, owner=root, group=root, mode=0644
# pets: post=/bin/systemctl daemon-reload
# pets: when=os:linux

[Unit]
Description=Run backup daily

[Timer]
OnCalendar=*-*-* 03:00:00
Persistent=true
```

## Development

### Prerequisites

- [Rust](https://rustup.rs/) 1.85+
- [pre-commit](https://pre-commit.com/)

### Setup

```bash
git clone https://github.com/butlerx/pets-configurator.git
cd pets-configurator
pre-commit install --hook-type pre-commit --hook-type commit-msg
cargo build
```

### Running checks

Pre-commit handles formatting, linting, and commit message validation
automatically. To run all hooks manually:

```bash
pre-commit run --all-files
```

Or run individual tools:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Commits must follow
[Conventional Commits](https://www.conventionalcommits.org/) format (enforced by
pre-commit).

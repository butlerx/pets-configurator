# PETS Configurator

[![Crates.io](https://img.shields.io/crates/v/pets-configurator)](https://crates.io/crates/pets-configurator)
[![CI](https://github.com/butlerx/pets-configurator/actions/workflows/rust.yml/badge.svg)](https://github.com/butlerx/pets-configurator/actions/workflows/rust.yml)
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

Create `~/pets/sudoers` with:

```sudoers
# pets: destfile=/etc/sudoers.d/myuser, owner=root, group=root, mode=0440
# pets: package=sudo
# pets: pre=/usr/sbin/visudo -cf
myuser ALL=(ALL:ALL) NOPASSWD:ALL
```

Preview what would happen:

```bash
sudo pets --dry-run
```

Apply:

```bash
sudo pets
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

### Vim configuration (symlink)

```vim
; pets: symlink=/root/.vimrc

syntax on
set expandtab
set shiftwidth=4
```

### SSH server

```sshd
# pets: destfile=/etc/ssh/sshd_config, owner=root, group=root, mode=0644
# pets: package=ssh
# pets: pre=/usr/sbin/sshd -t -f
# pets: post=/bin/systemctl reload ssh.service

PasswordAuthentication no
ChallengeResponseAuthentication no
UsePAM yes
```

### Firewall (ferm)

```
# pets: destfile=/etc/ferm/ferm.conf, owner=root, group=root, mode=644
# pets: package=ferm
# pets: pre=/usr/sbin/ferm -n
# pets: post=/bin/systemctl reload ferm.service

domain (ip ip6) {
    table filter {
        chain INPUT {
            policy DROP;
            mod state state INVALID DROP;
            mod state state (ESTABLISHED RELATED) ACCEPT;
            interface lo ACCEPT;
            proto icmp ACCEPT;
            proto tcp dport ssh ACCEPT;
        }
        chain OUTPUT  { policy ACCEPT; }
        chain FORWARD { policy DROP; }
    }
}
```

### Multiple packages with mixed managers

```
# pets: destfile=/etc/myapp.conf
# pets: package=curl
# pets: package=cargo:ripgrep

[myapp]
search_tool=rg
```

### Host-specific configuration

```
# pets: destfile=/etc/hostname, owner=root, group=root, mode=0644
# pets: when=hostname:webserver

webserver.example.com
```

### macOS-only Homebrew setup

```bash
# pets: destfile=/usr/local/etc/my-tool.conf
# pets: when=os:macos
# pets: package=brew:my-tool
# pets: post=/usr/bin/pkill -HUP my-tool

setting=value
```

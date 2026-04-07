# UUPM

**A command-line workflow for Unity dependencies.**

English | [中文](README.zh.md)

UUPM manages Unity registry sources, installs packages from Unity registries and NuGet, converts NuGet packages into Unity-consumable package layouts, and freezes project dependencies into local artifacts for offline or reproducible use.

## Why use UUPM

- **One CLI for two package ecosystems** - Use the same `install` command for Unity registry packages and NuGet packages.
- **Offline-ready workflows** - Embed a Unity registry package as a local `.tgz`, or freeze an entire project into local artifacts.
- **Unity-friendly NuGet import** - Convert `.nupkg` packages into `org.nuget.*` package folders with generated `.meta` files.
- **Centralized local config** - Keep registry definitions and Unity editor paths in a single user-level config file.

## Quick Start

Install from the repository root:

```bash
cargo install --path .
```

Or build a release binary:

```bash
cargo build --release
```

The binary will be available at `target/release/uupm` (`uupm.exe` on Windows).

Run project-level commands from the Unity project root, where the `Packages` directory exists. If `Packages/manifest.json` does not exist yet, the first Unity registry install will create a minimal manifest automatically.

## Common Workflows

### Install a Unity registry package

```bash
uupm install com.unity.ide.rider
uupm i com.example.tools@2.1.0
```

Without `-n`, UUPM resolves the package from a Unity registry and writes the selected version into `Packages/manifest.json`.

### Install and embed a Unity registry package

```bash
uupm i com.example.tools@2.1.0 --embed
```

This downloads `Packages/com.example.tools-2.1.0.tgz` and writes a `file:` dependency into the manifest instead of a registry version string.

### Install a NuGet package

```bash
uupm install -n Newtonsoft.Json
uupm i -n MyLibrary PrivateFeedName
```

With `-n` or `--nuget`, UUPM downloads the NuGet package, converts it into a Unity package layout under `Packages/`, and generates `.meta` files.

### Freeze project dependencies

```bash
uupm freeze
uupm f
```

UUPM resolves the current manifest, downloads registry packages as local `.tgz` files or copies built-in Unity packages, updates dependencies to `file:` references, and writes a backup to `Packages/manifest.src.json`.

### Manage registries

```bash
uupm registry add CustomUPM https://registry.example.com/npm
uupm registry default CustomUPM
uupm registry add NugetOrg https://api.nuget.org/v3/index.json -n
uupm registry default NugetOrg -n
```

### Manage Unity editor paths

```bash
uupm editor scan
uupm editor list
uupm editor default 2022.3.16f1
uupm editor add 2022.3.16f1 "C:\\Program Files\\Unity\\Hub\\Editor\\2022.3.16f1"
```

## Command Overview

### Top-level commands

| Command | Alias | Description |
|------|------|------|
| `install` | `i` | Install from a Unity registry or NuGet |
| `freeze` | `f` | Freeze manifest dependencies into local artifacts |
| `registry` | `r` | Manage package registries |
| `editor` | `e` | Manage Unity editor paths |

Global flags: `--help`, `--version`.

### `install`

```text
uupm install <name> [source]
```

- Default mode installs from a Unity registry.
- `name` supports `com.vendor.package` and `com.vendor.package@version`.
- `--embed` downloads a `.tgz` into `Packages` and writes a `file:` dependency.
- `-n` or `--nuget` switches the workflow to NuGet.
- `[source]` is only used with NuGet and refers to a configured source name in `~/.upmrc`.

### `registry`

| Subcommand | Alias | Description |
|--------|------|------|
| `add <name> <url>` | `a` | Add a registry |
| `remove <name>` | `r` | Remove a registry |
| `list` | `l` | List registries |
| `default <name>` | - | Set the default registry |

Use `-n` to operate on NuGet registries instead of Unity registries.

### `editor`

| Subcommand | Alias | Description |
|--------|------|------|
| `scan` | `s` | Scan common install locations and merge them into config |
| `add <name> <path>` | `a` | Register an editor path manually |
| `remove <name>` | `r` | Remove an editor record |
| `list` | `l` | List editor records |
| `default <name>` | - | Set the default editor key |

## Configuration

UUPM stores user-level configuration in `~/.upmrc`.

| Section | Purpose |
|--------|------|
| `registry.origin` | Unity registry source names, URLs, and the default source |
| `registry.nuget` | NuGet source names, index URLs, and the default source |
| `editor.version` | Unity version keys mapped to editor install paths |
| `editor.default` | Default Unity editor key used by workflows such as `freeze` |

The file is created automatically on first use. On Windows, `uupm editor scan` can populate common Unity install locations.

## Current Scope

- Manage Unity and NuGet registries
- Install Unity registry packages
- Install Unity registry packages as embedded local `.tgz` files
- Install NuGet packages as Unity package folders
- Freeze manifest dependencies into local artifacts
- Manage Unity editor paths and defaults

Not implemented yet:

- List installed packages
- Upgrade packages
- Remove packages
- Create package scaffolds
- Publish packages to registries

## License

[MIT](LICENSE)

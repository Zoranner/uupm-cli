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

### Add a Git dependency to the project manifest

```bash
uupm install com.vendor.mypkg --git https://github.com/org/repo.git
uupm i com.vendor.mypkg --git https://github.com/org/repo.git#v1.2.0
```

Writes the URL (and optional `#revision`) as the dependency value. Unity resolves the repository when you open the project in the Editor; UUPM does not clone or verify the URL.

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

Bearer tokens for private Unity registries (optional):

```bash
uupm registry token CustomUPM --token YOUR_TOKEN
uupm registry token CustomUPM --clear
# or when adding the registry:
uupm registry add CustomUPM https://registry.example.com/npm --token YOUR_TOKEN
```

The same token is sent on registry **GET** requests (`install`, `freeze`, `upgrade`, `info`, `search`) and on **publish** PUT, matched by `scopedRegistries[].name` in `manifest.json` or by registry URL in `~/.upmrc`.

### Publish a Unity package to a registry

From the package folder (must contain `package.json`):

```bash
uupm publish
uupm p ./path/to/com.vendor.mypkg -r CustomUPM
```

If you omit `-r`, the registry is chosen from `~/.upmrc` using scope rules (same as install). The request uses an npm-compatible PUT with a `package/` tarball; set a token on that registry if the server requires authentication.

Tarball contents respect `.npmignore` when present (git-style comments supported), and always skip common junk such as `.git`, `node_modules`, and `.npmignore` itself.

### Pack a folder to `.tgz` locally

```bash
uupm pack
uupm pack ./path/to/com.vendor.mypkg -o dist/my.tgz
```

Default output is `Packages/<name>-<version>.tgz` next to the current directory (same archive layout as `publish`).

### Create a package scaffold

```bash
uupm create com.vendor.mypkg --display-name "My Package" --author "You" --version 0.1.0
```

### List, upgrade, or remove manifest dependencies

```bash
uupm list
uupm upgrade
uupm upgrade com.vendor.mypkg
uupm remove com.vendor.mypkg
```

### Inspect or search a Unity registry

```bash
uupm info com.unity.addressables
uupm info com.vendor.mypkg -r CustomUPM
uupm search addressables
uupm s "my keywords" --limit 10
```

`info` fetches `{registry}/{package}` (same registry resolution as `install` when `Packages/manifest.json` exists). `search` calls npm-style `/-/v1/search`; many private Unity registries do not implement it and will return a non-success HTTP status.

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
| `remove` | `rm` | Remove a package from the manifest and local artifacts |
| `list` | `ls` | List packages in `Packages/manifest.json` |
| `upgrade` | `up` | Upgrade registry dependencies to the latest version |
| `create` | `c` | Create a new Unity package scaffold |
| `info` | - | Show package metadata from a Unity registry |
| `search` | `s` | Search packages (npm `/-/v1/search` when supported) |
| `pack` | - | Build a local `.tgz` from a package folder |
| `publish` | `p` | Publish a package directory to a Unity registry |
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
- `--git <url>` writes a Git URL dependency (optional `#revision`); cannot be combined with `-n`, `--embed`, or a NuGet `[source]` argument.
- `-n` or `--nuget` switches the workflow to NuGet.
- `[source]` is only used with NuGet and refers to a configured source name in `~/.upmrc`.

### `registry`

| Subcommand | Alias | Description |
|--------|------|------|
| `add <name> <url>` | `a` | Add a registry (`--token` for Unity only; `--scopes` for scoped routing) |
| `remove <name>` | `r` | Remove a registry |
| `list` | `l` | List registries |
| `default <name>` | - | Set the default registry |
| `token <name>` | - | Set `--token` or `--clear` on a Unity registry |

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
| `registry.origin` | Unity registry source names, URLs, optional per-source `token`, scopes, and the default source |
| `registry.nuget` | NuGet source names, index URLs, and the default source |
| `editor.version` | Unity version keys mapped to editor install paths |
| `editor.default` | Default Unity editor key used by workflows such as `freeze` |

The file is created automatically on first use. On Windows, `uupm editor scan` can populate common Unity install locations. You can also edit `token` under an origin source by hand; avoid committing secrets.

## Unity manifest version strings

Unity’s project `Packages/manifest.json` and each package’s `package.json` expect **plain SemVer** for registry dependencies (for example `1.2.3` or `1.0.0-preview.1`). They do **not** use npm range operators in those fields (`^`, `~`, `>=`, `*`, `||`, and so on). Resolution behavior uses `resolutionStrategy` and lock files, not npm-style strings in JSON.

UUPM writes **exact** versions on `install` (non-embed). `upgrade` skips entries that look like npm ranges; `freeze` fails with a clear error so you can fix the manifest. `list` marks suspicious values as `non-unity range?`.

## Current scope

- Manage Unity and NuGet registries (including optional Bearer tokens for Unity sources)
- Install Unity registry packages and embed them as local `.tgz` files
- Add Git URL dependencies to the project manifest
- Install NuGet packages as Unity package folders
- List, upgrade, and remove manifest dependencies
- Show registry metadata for a package (`info`) and optional npm search (`search`)
- Create Unity package scaffolds
- Pack a package folder to a `.tgz` without publishing
- Publish packages to npm-compatible Unity registries
- Freeze manifest dependencies into local artifacts
- Manage Unity editor paths and defaults

For the full command surface, run `uupm --help` and `uupm <command> --help`.

## License

[MIT](LICENSE)

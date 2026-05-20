# UUPM

**Unity dependency management from the command line.**

English | [中文](README.zh.md)

UUPM installs Unity registry packages, Git dependencies, and NuGet packages into Unity projects. It can also list, upgrade, remove, pack, publish, inspect, and freeze package dependencies.

Project-level commands should be run from the Unity project root. UUPM reads and writes `Packages/manifest.json`; if the manifest does not exist, registry installs create a minimal one.

## Quick Start

Install or update the latest GitHub Release:

```powershell
irm https://raw.githubusercontent.com/Zoranner/uupm-cli/master/install.ps1 | iex
```

```bash
curl -fsSL https://raw.githubusercontent.com/Zoranner/uupm-cli/master/install.sh | sh
```

Build from source instead:

```bash
cargo install --path .
```

Check the binary:

```bash
uupm --version
uupm --help
```

## First Use

UUPM stores user-level configuration in `~/.upmrc.toml`. The file is created on first use.

On Windows, scan installed Unity editors:

```bash
uupm editor scan
uupm editor list
uupm editor default 2022.3.16f1
```

Add registries when needed:

```bash
uupm registry add CustomUPM https://registry.example.com/npm --scopes com.vendor
uupm registry default CustomUPM
uupm registry add NugetOrg https://api.nuget.org/v3/index.json -n
uupm registry token CustomUPM --token YOUR_TOKEN
```

## Common Commands

```bash
# Unity registry package
uupm install com.unity.ide.rider
uupm install com.vendor.tool@1.2.3 --embed

# Git dependency
uupm install com.vendor.tool --git https://github.com/org/repo.git#v1.2.0

# NuGet package
uupm install -n Newtonsoft.Json

# Manifest maintenance
uupm list
uupm upgrade --dry-run
uupm upgrade
uupm remove com.vendor.tool
uupm doctor

# Package authoring and release support
uupm create com.vendor.tool --display-name "Vendor Tool"
uupm pack ./path/to/com.vendor.tool
uupm publish ./path/to/com.vendor.tool -r CustomUPM

# Registry lookup and offline artifacts
uupm info com.unity.addressables
uupm search addressables
uupm freeze
```

Run `uupm <command> --help` for command-specific options.

## Notes

- Unity registry dependencies are written as exact versions, not npm-style ranges such as `^1.2.3`.
- NuGet mode uses `-n` / `--nuget` and installs packages into Unity package folders.
- `search` and `publish` depend on the registry server supporting npm-compatible APIs.
- `freeze` rewrites dependencies to local `file:` artifacts and backs up the original manifest to `Packages/manifest.src.json`.

## License

[MIT](LICENSE)

# UUPM

`UUPM` is a tool for managing Unity packages, including installing, updating, and removing packages from UPM and NuGet registries.

## Installation

```bash
npm i -g @uupm/cli
```

## Usage

```bash
~ uupm -h
Usage: uupm [options] [command]

Options:
  -v, --version                        output the version number.
  -h, --help                           display help for command

Commands:
  install|i [options] <name> [source]  Install a package.
  freeze|f
  registry                             manage registries.
  editor                               manage unity editors.
```

```bash
~ uupm registry -h
Usage: uupm registry|r [options] [command]

manage registries.

Options:
  -h, --help                    display help for command

Commands:
  add|a [options] <name> <url>  add a new registry.
  remove|r [options] <name>     remove an existing registry.
  list|l [options]              list all registries.
  help [command]                display help for command
```

```bash
~ uupm editor -h
Usage: uupm editor|e [options] [command]

manage unity editors.

Options:
  -h, --help           display help for command

Commands:
  scan|s               scan current editor.
  add|a <name> <path>  add a new editor.
  remove|r <name>      remove an existing editor.
  list|l               list all editors.
  help [command]       display help for command
```

```bash
~ uupm install -h
Usage: uupm install|i [options] <name> [source]

install a package.

Arguments:
  name          package name to install.
  source        nuget package source name.

Options:
  -n, --nuget   install package from nuget.
  -s, --source  install package from source.
  -h, --help    display help for command
```

## Features

- [x] Configure UPM registries
- [x] Configure NuGet registries
- [ ] Install package from UPM
- [x] Install package from NuGet
- [ ] List installed packages
- [ ] Update package
- [ ] Remove package
- [x] Freeze version to offline mode
- [ ] Create new package
- [ ] Publish package to UPM

## License

MIT

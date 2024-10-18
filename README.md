# UUPM

`UUPM` is a tool for managing Unity packages, including installing, updating, and removing packages from UPM and NuGet registries.

## Installation

```bash
npm i -g @uupm/cli
```

## Usage

### Install package from UPM

```bash
uupm i <package-name>
```

```bash
uupm i <package-name>@<version>
```

### Install package from NuGet

```bash
uupm i <PackageName>
```

```bash
uupm i <PackageName>@<version>
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

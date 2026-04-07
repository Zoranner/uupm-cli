# UUPM

**面向 Unity 依赖管理的命令行工作流。**

[English](README.md) | 中文

UUPM 用于管理 Unity 注册表源、安装 Unity 注册表与 NuGet 上的依赖、将 NuGet 包转换为 Unity 可直接使用的包目录，并将项目依赖冻结为本地制品，以支持离线使用或可复现交付。

## 为什么使用 UUPM

- **一套命令覆盖两类包源**：Unity 注册表与 NuGet 共用 `install` 子命令。
- **支持离线工作流**：既可以将单个 Unity 包嵌入为本地 `.tgz`，也可以把整个项目依赖冻结为本地制品。
- **NuGet 到 Unity 的转换链路完整**：将 `.nupkg` 转换为 `org.nuget.*` 目录结构并自动生成 `.meta` 文件。
- **本地配置集中管理**：注册表定义与 Unity 编辑器路径统一保存在用户级配置中。

## 快速开始

在仓库根目录安装：

```bash
cargo install --path .
```

或构建发布二进制：

```bash
cargo build --release
```

生成的程序位于 `target/release/uupm`，Windows 下为 `uupm.exe`。

与工程相关的命令应在 Unity 工程根目录执行，并保证存在 `Packages` 目录。若尚未创建 `Packages/manifest.json`，首次从 Unity 注册表安装时会自动生成最小清单。

## 常见工作流

### 安装 Unity 注册表包

```bash
uupm install com.unity.ide.rider
uupm i com.example.tools@2.1.0
```

不带 `-n` 时，UUPM 会从 Unity 注册表解析包版本，并将结果写入 `Packages/manifest.json`。

### 安装并嵌入 Unity 注册表包

```bash
uupm i com.example.tools@2.1.0 --embed
```

该模式会下载 `Packages/com.example.tools-2.1.0.tgz`，并在清单中写入 `file:` 依赖，而不是注册表版本号。

### 安装 NuGet 包

```bash
uupm install -n Newtonsoft.Json
uupm i -n MyLibrary PrivateFeedName
```

带 `-n` 或 `--nuget` 时，UUPM 会下载 NuGet 包，将其转换为 Unity 包目录，并生成 `.meta` 文件。

### 冻结项目依赖

```bash
uupm freeze
uupm f
```

UUPM 会解析当前清单，将注册表包下载为本地 `.tgz` 或拷贝 Unity 内置包，并将依赖更新为 `file:` 引用，同时备份原始清单到 `Packages/manifest.src.json`。

### 管理注册表

```bash
uupm registry add CustomUPM https://registry.example.com/npm
uupm registry default CustomUPM
uupm registry add NugetOrg https://api.nuget.org/v3/index.json -n
uupm registry default NugetOrg -n
```

### 管理 Unity 编辑器路径

```bash
uupm editor scan
uupm editor list
uupm editor default 2022.3.16f1
uupm editor add 2022.3.16f1 "C:\\Program Files\\Unity\\Hub\\Editor\\2022.3.16f1"
```

## 命令总览

### 顶层命令

| 命令 | 别名 | 说明 |
|------|------|------|
| `install` | `i` | 从 Unity 注册表或 NuGet 安装 |
| `freeze` | `f` | 将清单依赖冻结为本地制品 |
| `registry` | `r` | 管理包注册表 |
| `editor` | `e` | 管理 Unity 编辑器路径 |

全局参数：`--help`、`--version`。

### `install`

```text
uupm install <name> [source]
```

- 默认模式从 Unity 注册表安装。
- `name` 支持 `com.vendor.package` 与 `com.vendor.package@version`。
- `--embed` 会将包下载为 `Packages` 下的 `.tgz` 并写入 `file:` 依赖。
- `-n` 或 `--nuget` 会切换到 NuGet 安装流程。
- `[source]` 仅在 NuGet 模式下生效，表示 `~/.upmrc` 中已配置的源名称。

### `registry`

| 子命令 | 别名 | 说明 |
|--------|------|------|
| `add <name> <url>` | `a` | 添加注册表 |
| `remove <name>` | `r` | 删除注册表 |
| `list` | `l` | 列出注册表 |
| `default <name>` | - | 设置默认注册表 |

追加 `-n` 可操作 NuGet 注册表；省略时操作 Unity 注册表。

### `editor`

| 子命令 | 别名 | 说明 |
|--------|------|------|
| `scan` | `s` | 扫描常见安装位置并合并到配置 |
| `add <name> <path>` | `a` | 手动登记编辑器路径 |
| `remove <name>` | `r` | 删除编辑器记录 |
| `list` | `l` | 列出编辑器记录 |
| `default <name>` | - | 设置默认编辑器键 |

## 配置

UUPM 的用户级配置文件位于 `~/.upmrc`。

| 配置段 | 作用 |
|--------|------|
| `registry.origin` | Unity 注册表名称、地址与默认源 |
| `registry.nuget` | NuGet 源名称、索引地址与默认源 |
| `editor.version` | Unity 版本键与安装路径映射 |
| `editor.default` | 冻结依赖等流程使用的默认编辑器键 |

首次使用时会自动创建该文件。在 Windows 上，`uupm editor scan` 可用于填充常见 Unity 安装路径。

## 当前范围

- 管理 Unity 与 NuGet 注册表
- 安装 Unity 注册表包
- 将 Unity 注册表包嵌入为本地 `.tgz`
- 将 NuGet 包安装为 Unity 包目录
- 冻结清单依赖为本地制品
- 管理 Unity 编辑器路径与默认值

暂未实现：

- 列出已安装包
- 升级包
- 删除包
- 创建包脚手架
- 发布包到注册表

## 许可证

[MIT](LICENSE)

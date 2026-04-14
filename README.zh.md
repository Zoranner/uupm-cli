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

### 在工程清单中添加 Git 依赖

```bash
uupm install com.vendor.mypkg --git https://github.com/org/repo.git
uupm i com.vendor.mypkg --git https://github.com/org/repo.git#v1.2.0
```

将 URL（及可选的 `#revision`）写入 `dependencies`。Unity 在编辑器中解析仓库；UUPM 不会克隆或校验 URL。

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

私有 Unity 注册表可配置 Bearer 令牌（可选）：

```bash
uupm registry token CustomUPM --token YOUR_TOKEN
uupm registry token CustomUPM --clear
# 或在添加注册表时：
uupm registry add CustomUPM https://registry.example.com/npm --token YOUR_TOKEN
```

该令牌会用于注册表的 **GET**（`install`、`freeze`、`upgrade`、`info`、`search`）以及 **publish** 的 PUT；按 `manifest.json` 里 `scopedRegistries[].name` 或 `~/.upmrc` 中的注册表 URL 与源配置对齐后自动附加。

### 将 Unity 包发布到注册表

在包目录下执行（需存在 `package.json`）：

```bash
uupm publish
uupm p ./path/to/com.vendor.mypkg -r CustomUPM
```

省略 `-r` 时，会按与安装相同的 scope 规则从 `~/.upmrc` 选择注册表。请求体为 npm 兼容的 PUT，并附带 `package/` 前缀的 tarball；若服务端需要鉴权，请为该注册表配置 `token`。

打包时会读取目录下的 `.npmignore`（支持 `#` 注释），并始终排除 `.git`、`node_modules` 等常见无关内容以及根目录的 `.npmignore` 文件本身。

### 本地打成 `.tgz`

```bash
uupm pack
uupm pack ./path/to/com.vendor.mypkg -o dist/my.tgz
```

默认输出为当前目录下的 `Packages/<name>-<version>.tgz`，归档布局与 `publish` 一致，不访问网络。

### 创建包脚手架

```bash
uupm create com.vendor.mypkg --display-name "My Package" --author "You" --version 0.1.0
```

### 列出、升级或移除清单依赖

```bash
uupm list
uupm upgrade
uupm upgrade com.vendor.mypkg
uupm remove com.vendor.mypkg
```

### 查看注册表包信息或搜索

```bash
uupm info com.unity.addressables
uupm info com.vendor.mypkg -r CustomUPM
uupm search addressables
uupm s "关键词" --limit 10
```

`info` 请求 `{registry}/{package}`；若当前目录存在 `Packages/manifest.json`，注册表选择与 `install` 一致（含 scoped）。`search` 使用 npm 的 `/-/v1/search`，不少私有 Unity 源未实现，会返回非 2xx。

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
| `remove` | `rm` | 从清单移除包并清理本地制品 |
| `list` | `ls` | 列出 `Packages/manifest.json` 中的依赖 |
| `upgrade` | `up` | 将注册表依赖升级到最新版本 |
| `create` | `c` | 创建 Unity 包脚手架 |
| `info` | - | 查看 Unity 注册表上某包的元数据 |
| `search` | `s` | 搜索包（需注册表支持 npm `/-/v1/search`） |
| `pack` | - | 将包目录打成本地 `.tgz` |
| `publish` | `p` | 将包目录发布到 Unity 注册表 |
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
- `--git <url>` 写入 Git URL 依赖（可选 `#revision`）；不能与 `-n`、`--embed` 或 NuGet 的 `[source]` 参数同时使用。
- `-n` 或 `--nuget` 会切换到 NuGet 安装流程。
- `[source]` 仅在 NuGet 模式下生效，表示 `~/.upmrc` 中已配置的源名称。

### `registry`

| 子命令 | 别名 | 说明 |
|--------|------|------|
| `add <name> <url>` | `a` | 添加注册表（Unity 源可用 `--token`；路由可用 `--scopes`） |
| `remove <name>` | `r` | 删除注册表 |
| `list` | `l` | 列出注册表 |
| `default <name>` | - | 设置默认注册表 |
| `token <name>` | - | 为 Unity 注册表设置 `--token` 或 `--clear` |

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
| `registry.origin` | Unity 注册表名称、地址、各源可选 `token`、scope 与默认源 |
| `registry.nuget` | NuGet 源名称、索引地址与默认源 |
| `editor.version` | Unity 版本键与安装路径映射 |
| `editor.default` | 冻结依赖等流程使用的默认编辑器键 |

首次使用时会自动创建该文件。在 Windows 上，`uupm editor scan` 可用于填充常见 Unity 安装路径。也可直接编辑各 origin 源下的 `token`；请勿将密钥提交到版本库。

## Unity 清单里的版本写法

工程的 `Packages/manifest.json` 与各包的 `package.json` 中，对**注册表**依赖应使用 **普通 SemVer 字符串**（如 `1.2.3`、`1.0.0-preview.1`），**不是** npm 的区间运算符（`^`、`~`、`>=`、`*`、`||` 等）。具体解析由 Unity 的 `resolutionStrategy`、锁文件等机制完成，而不是在 JSON 里写 npm 式区间。

UUPM 在非嵌入安装时会写入**精确**版本。`upgrade` 会跳过疑似 npm 区间的项；`freeze` 会直接报错提示修改清单。`list` 会将可疑依赖标为 `non-unity range?`。

## 当前范围

- 管理 Unity 与 NuGet 注册表（含 Unity 源可选 Bearer 令牌）
- 安装 Unity 注册表包，并支持嵌入为本地 `.tgz`
- 在工程清单中添加 Git URL 依赖
- 将 NuGet 包安装为 Unity 包目录
- 列出、升级、移除清单依赖
- 查看注册表包信息（`info`）与可选的 npm 搜索（`search`）
- 创建 Unity 包脚手架
- 将包目录打成 `.tgz`（仅本地，不发布）
- 向 npm 兼容的 Unity 注册表发布包
- 冻结清单依赖为本地制品
- 管理 Unity 编辑器路径与默认值

完整子命令与参数请使用 `uupm --help` 与 `uupm <命令> --help` 查看。

## 许可证

[MIT](LICENSE)

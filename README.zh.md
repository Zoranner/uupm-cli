# UUPM

**Unity 依赖管理命令行工具。**

[English](README.md) | 中文

UUPM 用来在 Unity 工程中安装 Unity 注册表包、Git 依赖和 NuGet 包，也支持依赖列表、升级、移除、打包、发布、查询和冻结。

工程相关命令应在 Unity 工程根目录执行。UUPM 会读写 `Packages/manifest.json`；如果清单不存在，首次注册表安装会创建最小清单。

## 快速开始

安装或更新最新 GitHub Release：

```powershell
irm https://raw.githubusercontent.com/Zoranner/uupm-cli/master/install.ps1 | iex
```

```bash
curl -fsSL https://raw.githubusercontent.com/Zoranner/uupm-cli/master/install.sh | sh
```

也可以从源码安装：

```bash
cargo install --path .
```

检查命令：

```bash
uupm --version
uupm --help
```

## 首次使用

UUPM 的用户级配置文件是 `~/.upmrc.toml`，首次使用时自动创建。

Windows 下可以先扫描 Unity 编辑器：

```bash
uupm editor scan
uupm editor list
uupm editor default 2022.3.16f1
```

按需添加注册表：

```bash
uupm registry add CustomUPM https://registry.example.com/npm --scopes com.vendor
uupm registry default CustomUPM
uupm registry add NugetOrg https://api.nuget.org/v3/index.json -n
uupm registry token CustomUPM --token YOUR_TOKEN
```

## 常用命令

```bash
# Unity 注册表包
uupm install com.unity.ide.rider
uupm install com.vendor.tool@1.2.3 --embed

# Git 依赖
uupm install com.vendor.tool --git https://github.com/org/repo.git#v1.2.0

# NuGet 包
uupm install -n Newtonsoft.Json

# 清单维护
uupm list
uupm upgrade --dry-run
uupm upgrade
uupm remove com.vendor.tool
uupm doctor

# 包开发与发布
uupm create com.vendor.tool --display-name "Vendor Tool"
uupm pack ./path/to/com.vendor.tool
uupm publish ./path/to/com.vendor.tool -r CustomUPM

# 注册表查询与离线制品
uupm info com.unity.addressables
uupm search addressables
uupm freeze
```

具体参数用 `uupm <command> --help` 查看。

## 注意事项

- Unity 注册表依赖会写入精确版本，不写 `^1.2.3` 这类 npm 区间。
- NuGet 模式使用 `-n` / `--nuget`，会把包安装成 Unity 包目录。
- `search` 和 `publish` 依赖注册表服务端支持 npm 兼容接口。
- `freeze` 会把依赖改写成本地 `file:` 制品，并将原始清单备份到 `Packages/manifest.src.json`。

## 许可证

[MIT](LICENSE)

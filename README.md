# UUPM

用于管理 Unity 工程包依赖的命令行工具：维护 Unity 包管理器与 NuGet 注册表、安装包、将 NuGet 程序集转为 Unity 包、冻结依赖为本地压缩包。

## 安装

自本仓库安装或构建：

```bash
cargo install --path .
```

```bash
cargo build --release
```

后者产物为 `target/release/uupm`（Windows 为 `uupm.exe`），请加入 `PATH` 或复制到常用目录。需要本机已安装 Rust 工具链。

与清单相关的命令请在 Unity 工程根目录执行（含 `Packages` 目录）。全局配置位于 **`~/.upmrc`**，首次使用时会自动生成；Windows 下可用 `uupm editor scan` 尝试扫描编辑器路径。

## 用法

下列为与 `uupm --help` 及各子命令帮助一致的结构说明（中文释义）。

```text
用法: uupm [命令]

命令:
  install|i [选项] <名称> [源]  安装包（默认可从 Unity 注册表安装，-n 时从 NuGet 安装）
  freeze|f                      冻结依赖为本地包
  registry|r                    管理注册表
  editor|e                      管理 Unity 编辑器路径

选项:
  -h, --help     显示帮助
  -V, --version  显示版本

无子命令时打印标识横幅。
```

```text
用法: uupm registry|r <命令>

说明: 管理注册表。

命令:
  add|a [选项] <名称> <地址>   添加注册表
  remove|r [选项] <名称>     删除注册表
  list|l [选项]               列出注册表
  default [选项] <名称>       将已存在的源设为默认

选项:
  -n, --nuget   操作对象为 NuGet 注册表（省略则为 Unity 包管理器源）
  -h, --help    显示帮助
```

```text
用法: uupm editor|e <命令>

说明: 管理 Unity 编辑器安装路径。

命令:
  scan|s               扫描本机常见安装目录并写入配置
  add|a <名称> <路径>  添加一条编辑器记录
  remove|r <名称>      删除一条记录
  list|l               列出记录
  default <名称>       设置默认编辑器版本键

选项:
  -h, --help           显示帮助
```

```text
用法: uupm install|i [选项] <名称> [源]

说明: 安装包。

参数:
  名称   包名；Unity 注册表安装时为 com.xxx 形式，可写 @版本；NuGet 时为包 ID，可写 @版本
  源     可选；仅在使用 -n 时表示 .upmrc 中已配置的 NuGet 源名称

选项:
  -n, --nuget   从 NuGet 安装
  --embed      从 Unity 注册表安装时，将包下载为 Packages 下 .tgz 并写 file: 引用（不可与 -n 同用）
  -h, --help   显示帮助
```

示例：

```bash
uupm install com.unity.ide.rider
uupm i com.example.tools@2.1.0 --embed
uupm install -n Newtonsoft.Json
uupm r add MyUPM https://example.com/npm
uupm registry default MyUPM
uupm e scan
uupm freeze
```

## 功能

- [x] 配置 Unity 包管理器注册表
- [x] 配置 NuGet 注册表
- [x] 设置默认注册表名称
- [x] 从 Unity 注册表安装包
- [x] 从 NuGet 安装包
- [x] 安装时可选将 Unity 注册表包嵌入为本地压缩包
- [x] 冻结依赖为离线模式
- [x] 配置与扫描 Unity 编辑器路径
- [x] 设置默认编辑器版本键
- [ ] 列出已安装包
- [ ] 升级包
- [ ] 移除包
- [ ] 新建包脚手架
- [ ] 发布包到注册表

## 许可证

MIT

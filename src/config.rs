use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

// ---------------------------------------------------------------------------
// Config structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub registry: RegistrySection,
    pub editor: EditorSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrySection {
    pub origin: OriginGroup,
    pub nuget: NugetGroup,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OriginGroup {
    pub default: String,
    #[serde(default)]
    pub sources: BTreeMap<String, OriginSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OriginSource {
    pub url: String,
    /// Scope prefixes this registry handles. Empty = default/catch-all.
    #[serde(default)]
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NugetGroup {
    pub default: String,
    #[serde(default)]
    pub sources: BTreeMap<String, NugetSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NugetSource {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorSection {
    pub default: String,
    #[serde(default)]
    pub versions: BTreeMap<String, String>,
}

// ---------------------------------------------------------------------------
// Config file path
// ---------------------------------------------------------------------------

fn config_path() -> Result<std::path::PathBuf> {
    let home = dirs::home_dir().context("cannot resolve home directory")?;
    Ok(home.join(".upmrc.toml"))
}

// ---------------------------------------------------------------------------
// Read / write
// ---------------------------------------------------------------------------

pub fn read_configs() -> Result<GlobalConfig> {
    let path = config_path()?;
    if !path.exists() {
        let data = init_configs()?;
        write_configs(&data)?;
        return Ok(data);
    }
    let raw = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    toml::from_str::<GlobalConfig>(&raw)
        .context("invalid ~/.upmrc.toml — fix or delete it to reset")
}

pub fn write_configs(configs: &GlobalConfig) -> Result<()> {
    let path = config_path()?;
    let content = toml::to_string_pretty(configs)?;
    fs::write(&path, content).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Editor scanning
// ---------------------------------------------------------------------------

#[cfg(windows)]
fn editor_base_dirs() -> Vec<std::path::PathBuf> {
    // Unity Hub 默认编辑器安装目录，每个子目录为一个版本号
    [r"C:\Program Files\Unity\Hub\Editor\"]
        .into_iter()
        .map(std::path::PathBuf::from)
        .collect()
}

#[cfg(not(windows))]
fn editor_base_dirs() -> Vec<std::path::PathBuf> {
    vec![]
}

/// 扫描 `C:\Program Files\` 下所有以 `Unity ` 开头的目录（手动安装格式：`Unity 2022.3.52f1`）
#[cfg(windows)]
fn scan_manual_install_dirs() -> Vec<std::path::PathBuf> {
    let base = std::path::Path::new(r"C:\Program Files");
    let Ok(entries) = fs::read_dir(base) else {
        return vec![];
    };
    entries
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            let name = p.file_name()?.to_str()?;
            if p.is_dir() && name.starts_with("Unity ") {
                Some(p)
            } else {
                None
            }
        })
        .collect()
}

#[cfg(not(windows))]
fn scan_manual_install_dirs() -> Vec<std::path::PathBuf> {
    vec![]
}

#[cfg(windows)]
fn unity_exe_path(editor_path: &Path) -> std::path::PathBuf {
    editor_path.join("Editor").join("Unity.exe")
}

#[cfg(not(windows))]
fn unity_exe_path(editor_path: &Path) -> std::path::PathBuf {
    editor_path.join("Editor").join("Unity")
}

#[cfg(windows)]
fn read_unity_product_version(exe: &Path) -> Option<String> {
    // 通过环境变量传递路径，彻底避免任何注入风险
    let out = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-Item -LiteralPath $env:UNITY_EXE_PATH).VersionInfo.ProductVersion",
        ])
        .env("UNITY_EXE_PATH", exe)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    let v = s.trim();
    if v.is_empty() {
        return None;
    }
    Some(v.split('_').next().unwrap_or(v).to_string())
}

#[cfg(not(windows))]
fn read_unity_product_version(_exe: &Path) -> Option<String> {
    None
}

pub fn scan_editor_versions() -> BTreeMap<String, String> {
    let mut versions = BTreeMap::new();

    // Hub 管理的编辑器：枚举 Hub\Editor\ 下的版本子目录
    for base in editor_base_dirs() {
        if !base.is_dir() {
            continue;
        }
        let Ok(entries) = fs::read_dir(&base) else {
            continue;
        };
        for entry in entries.flatten() {
            let child = entry.path();
            if !child.is_dir() {
                continue;
            }
            let unity_exe = unity_exe_path(&child);
            if !unity_exe.is_file() {
                continue;
            }
            if let Some(ver) = read_unity_product_version(&unity_exe) {
                versions.insert(ver, child.to_string_lossy().to_string());
            }
        }
    }

    // 手动安装的编辑器：`C:\Program Files\Unity 2022.3.52f1\Editor\Unity.exe`
    for dir in scan_manual_install_dirs() {
        let unity_exe = unity_exe_path(&dir);
        if !unity_exe.is_file() {
            continue;
        }
        if let Some(ver) = read_unity_product_version(&unity_exe) {
            versions.insert(ver, dir.to_string_lossy().to_string());
        }
    }

    versions
}

// ---------------------------------------------------------------------------
// Init defaults
// ---------------------------------------------------------------------------

pub fn init_configs() -> Result<GlobalConfig> {
    let editor_versions = scan_editor_versions();
    let default_editor = editor_versions.keys().next().cloned().unwrap_or_default();

    let mut origin_sources = BTreeMap::new();
    origin_sources.insert(
        "Unity".to_string(),
        OriginSource {
            url: "https://packages.unity.com".to_string(),
            scopes: vec![],
        },
    );

    let mut nuget_sources = BTreeMap::new();
    nuget_sources.insert(
        "NuGet".to_string(),
        NugetSource {
            url: "https://api.nuget.org/v3/index.json".to_string(),
        },
    );

    Ok(GlobalConfig {
        registry: RegistrySection {
            origin: OriginGroup {
                default: "Unity".to_string(),
                sources: origin_sources,
            },
            nuget: NugetGroup {
                default: "NuGet".to_string(),
                sources: nuget_sources,
            },
        },
        editor: EditorSection {
            default: default_editor,
            versions: editor_versions,
        },
    })
}

// ---------------------------------------------------------------------------
// Registry helpers
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryKind {
    Origin,
    Nuget,
}

pub fn add_registry(name: &str, url: &str, scopes: Vec<String>, kind: RegistryKind) -> Result<()> {
    let mut c = read_configs()?;
    match kind {
        RegistryKind::Origin => {
            c.registry.origin.sources.insert(
                name.to_string(),
                OriginSource { url: url.to_string(), scopes },
            );
        }
        RegistryKind::Nuget => {
            c.registry.nuget.sources.insert(
                name.to_string(),
                NugetSource { url: url.to_string() },
            );
        }
    }
    write_configs(&c)?;
    Ok(())
}

pub fn remove_registry(name: &str, kind: RegistryKind) -> Result<()> {
    let mut c = read_configs()?;
    match kind {
        RegistryKind::Origin => { c.registry.origin.sources.remove(name); }
        RegistryKind::Nuget => { c.registry.nuget.sources.remove(name); }
    }
    write_configs(&c)?;
    Ok(())
}

pub fn set_default_registry(name: &str, kind: RegistryKind) -> Result<()> {
    let mut c = read_configs()?;
    match kind {
        RegistryKind::Origin => {
            if !c.registry.origin.sources.contains_key(name) {
                bail!("unknown origin registry {:?}", name);
            }
            c.registry.origin.default = name.to_string();
        }
        RegistryKind::Nuget => {
            if !c.registry.nuget.sources.contains_key(name) {
                bail!("unknown nuget registry {:?}", name);
            }
            c.registry.nuget.default = name.to_string();
        }
    }
    write_configs(&c)?;
    Ok(())
}

pub fn list_registries(kind: RegistryKind) -> Result<()> {
    let c = read_configs()?;
    match kind {
        RegistryKind::Origin => println!("{}", toml::to_string_pretty(&c.registry.origin.sources)?),
        RegistryKind::Nuget => println!("{}", toml::to_string_pretty(&c.registry.nuget.sources)?),
    }
    Ok(())
}

/// Find the best registry URL for a given package name.
/// Checks scoped registries first (most specific scope wins), then falls back to default.
pub fn resolve_origin_registry<'a>(
    package_name: &str,
    config: &'a GlobalConfig,
) -> Result<(&'a str, &'a OriginSource)> {
    let origin = &config.registry.origin;

    // 1. 找 scopes 最长前缀匹配（最具体的优先）
    let mut best: Option<(&str, &OriginSource, usize)> = None;
    for (name, src) in &origin.sources {
        for scope in &src.scopes {
            if package_name.starts_with(scope.as_str()) {
                let len = scope.len();
                if best.as_ref().map_or(true, |&(_, _, best_len)| len > best_len) {
                    best = Some((name.as_str(), src, len));
                }
            }
        }
    }
    if let Some((name, src, _)) = best {
        return Ok((name, src));
    }

    // 2. 回退到默认源
    let default_name = &origin.default;
    origin
        .sources
        .get(default_name)
        .map(|src| (default_name.as_str(), src))
        .with_context(|| format!("default origin registry {:?} not found in ~/.upmrc.toml", default_name))
}

// ---------------------------------------------------------------------------
// Editor helpers
// ---------------------------------------------------------------------------

pub fn add_editor(name: &str, path: &str) -> Result<()> {
    let mut c = read_configs()?;
    c.editor.versions.insert(name.to_string(), path.to_string());
    write_configs(&c)?;
    Ok(())
}

pub fn remove_editor(name: &str) -> Result<()> {
    let mut c = read_configs()?;
    c.editor.versions.remove(name);
    write_configs(&c)?;
    Ok(())
}

pub fn set_default_editor(name: &str) -> Result<()> {
    let mut c = read_configs()?;
    if !c.editor.versions.contains_key(name) {
        bail!("unknown editor version key {:?}", name);
    }
    c.editor.default = name.to_string();
    write_configs(&c)?;
    Ok(())
}

pub fn list_editors() -> Result<()> {
    let c = read_configs()?;
    println!("{}", toml::to_string_pretty(&c.editor.versions)?);
    Ok(())
}

pub fn scan_and_merge_editors() -> Result<()> {
    let mut c = read_configs()?;
    for (k, v) in scan_editor_versions() {
        c.editor.versions.insert(k, v);
    }
    if c.editor.default.is_empty() {
        c.editor.default = c.editor.versions.keys().next().cloned().unwrap_or_default();
    }
    write_configs(&c)?;
    println!("Editor entries updated.");
    Ok(())
}

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub registry: RegistrySection,
    pub editor: EditorSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrySection {
    pub origin: RegistrySourceBlock,
    pub nuget: RegistrySourceBlock,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrySourceBlock {
    pub default: String,
    pub source: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorSection {
    pub default: String,
    #[serde(default)]
    pub version: BTreeMap<String, String>,
}

fn config_path() -> Result<std::path::PathBuf> {
    let home = dirs::home_dir().context("cannot resolve home directory")?;
    Ok(home.join(".upmrc"))
}

pub fn read_configs() -> Result<GlobalConfig> {
    let path = config_path()?;
    if !path.exists() {
        let data = init_configs()?;
        write_configs(&data)?;
        return Ok(data);
    }
    let raw = fs::read_to_string(&path).with_context(|| format!("read {:?}", path))?;
    match serde_yaml::from_str::<GlobalConfig>(&raw) {
        Ok(c) => Ok(c),
        Err(e) => {
            anyhow::bail!("invalid .upmrc: {}\nFix or delete ~/.upmrc to reset.", e)
        }
    }
}

pub fn write_configs(configs: &GlobalConfig) -> Result<()> {
    let path = config_path()?;
    let yaml = serde_yaml::to_string(configs)?;
    fs::write(&path, yaml).with_context(|| format!("write {:?}", path))?;
    Ok(())
}

/// Windows paths aligned with the original Node implementation.
#[cfg(windows)]
fn editor_base_dirs() -> Vec<std::path::PathBuf> {
    // Unity Hub 默认编辑器安装目录，每个子目录为一个版本号
    [r"C:\Program Files\Unity\Hub\Editor\"]
        .into_iter()
        .map(std::path::PathBuf::from)
        .collect()
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

#[cfg(not(windows))]
fn editor_base_dirs() -> Vec<std::path::PathBuf> {
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

pub fn init_configs() -> Result<GlobalConfig> {
    let editor_version = scan_editor_versions();
    let default_editor = editor_version.keys().next().cloned().unwrap_or_default();

    let mut origin_sources = BTreeMap::new();
    origin_sources.insert(
        "Unity".to_string(),
        "https://packages.unity.com".to_string(),
    );

    let mut nuget_sources = BTreeMap::new();
    nuget_sources.insert(
        "Nuget".to_string(),
        "https://api.nuget.org/v3/index.json".to_string(),
    );

    Ok(GlobalConfig {
        registry: RegistrySection {
            origin: RegistrySourceBlock {
                default: "Unity".to_string(),
                source: origin_sources,
            },
            nuget: RegistrySourceBlock {
                default: "Nuget".to_string(),
                source: nuget_sources,
            },
        },
        editor: EditorSection {
            default: default_editor,
            version: editor_version,
        },
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryKind {
    Origin,
    Nuget,
}

pub fn add_registry(name: &str, url: &str, kind: RegistryKind) -> Result<()> {
    let mut c = read_configs()?;
    match kind {
        RegistryKind::Origin => {
            c.registry
                .origin
                .source
                .insert(name.to_string(), url.to_string());
        }
        RegistryKind::Nuget => {
            c.registry
                .nuget
                .source
                .insert(name.to_string(), url.to_string());
        }
    }
    write_configs(&c)?;
    Ok(())
}

pub fn remove_registry(name: &str, kind: RegistryKind) -> Result<()> {
    let mut c = read_configs()?;
    match kind {
        RegistryKind::Origin => {
            c.registry.origin.source.remove(name);
        }
        RegistryKind::Nuget => {
            c.registry.nuget.source.remove(name);
        }
    }
    write_configs(&c)?;
    Ok(())
}

pub fn set_default_origin_registry(name: &str) -> Result<()> {
    let mut c = read_configs()?;
    if !c.registry.origin.source.contains_key(name) {
        bail!("unknown UPM registry name {:?}", name);
    }
    c.registry.origin.default = name.to_string();
    write_configs(&c)?;
    Ok(())
}

pub fn set_default_nuget_registry(name: &str) -> Result<()> {
    let mut c = read_configs()?;
    if !c.registry.nuget.source.contains_key(name) {
        bail!("unknown NuGet registry name {:?}", name);
    }
    c.registry.nuget.default = name.to_string();
    write_configs(&c)?;
    Ok(())
}

pub fn list_registries(kind: RegistryKind) -> Result<()> {
    let c = read_configs()?;
    match kind {
        RegistryKind::Origin => println!(
            "{}",
            serde_json::to_string_pretty(&c.registry.origin.source)?
        ),
        RegistryKind::Nuget => println!(
            "{}",
            serde_json::to_string_pretty(&c.registry.nuget.source)?
        ),
    }
    Ok(())
}

pub fn default_origin_registry_url(config: &GlobalConfig) -> Result<String> {
    let name = &config.registry.origin.default;
    config
        .registry
        .origin
        .source
        .get(name)
        .cloned()
        .with_context(|| format!("default origin registry {:?} missing in .upmrc", name))
}

pub fn add_editor(name: &str, path: &str) -> Result<()> {
    let mut c = read_configs()?;
    c.editor.version.insert(name.to_string(), path.to_string());
    write_configs(&c)?;
    Ok(())
}

pub fn remove_editor(name: &str) -> Result<()> {
    let mut c = read_configs()?;
    c.editor.version.remove(name);
    write_configs(&c)?;
    Ok(())
}

pub fn set_default_editor(name: &str) -> Result<()> {
    let mut c = read_configs()?;
    if !c.editor.version.contains_key(name) {
        bail!("unknown editor version key {:?}", name);
    }
    c.editor.default = name.to_string();
    write_configs(&c)?;
    Ok(())
}

pub fn list_editors() -> Result<()> {
    let c = read_configs()?;
    println!("{}", serde_json::to_string_pretty(&c.editor.version)?);
    Ok(())
}

/// Rescan install folders and merge into `editor.version` (keeps manual entries unless overwritten by same version key).
pub fn scan_and_merge_editors() -> Result<()> {
    let mut c = read_configs()?;
    for (k, v) in scan_editor_versions() {
        c.editor.version.insert(k, v);
    }
    if c.editor.default.is_empty() {
        c.editor.default = c.editor.version.keys().next().cloned().unwrap_or_default();
    }
    write_configs(&c)?;
    println!("Editor entries updated.");
    Ok(())
}

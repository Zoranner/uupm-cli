use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub const MANIFEST_PATH: &str = "Packages/manifest.json";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScopedRegistry {
    pub name: String,
    pub url: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct RegistryPackageBody {
    #[serde(default)]
    pub versions: BTreeMap<String, VersionDetailJson>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct VersionDetailJson {
    #[serde(default)]
    pub dependencies: BTreeMap<String, String>,
    pub dist: DistJson,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DistJson {
    pub tarball: String,
}

pub fn load_manifest_value(path: impl AsRef<Path>) -> Result<Value> {
    let p = path.as_ref();
    let raw = fs::read_to_string(p).with_context(|| format!("read {}", p.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parse {}", p.display()))
}

pub fn save_manifest_pretty(path: impl AsRef<Path>, v: &Value) -> Result<()> {
    let p = path.as_ref();
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(p, serde_json::to_string_pretty(v)?).with_context(|| format!("write {}", p.display()))
}

pub fn empty_manifest_object() -> Value {
    json!({
        "dependencies": {}
    })
}

/// Returns true if `dependencies` value looks like an **npm-style range** (`^`, `~`, `>=`, `*`, `||`, …).
///
/// Unity [project](https://docs.unity3d.com/6000.4/Documentation/Manual/upm-manifestPrj.html) and
/// [package](https://docs.unity3d.com/6000.4/Documentation/Manual/upm-manifestPkg.html) manifests
/// expect plain SemVer strings for registry packages (no npm range operators). `file:`, `git:`, and
/// `https://` sources are not checked here.
pub fn looks_like_npm_style_version_range(version_field: &str) -> bool {
    let s = version_field.trim();
    if s.starts_with("file:") || s.starts_with("git:") || s.starts_with("https://") {
        return false;
    }
    if s.starts_with('^') || s.starts_with('~') {
        return true;
    }
    if s.starts_with(">=") || s.starts_with("<=") {
        return true;
    }
    let b = s.as_bytes();
    if b.first() == Some(&b'>') || b.first() == Some(&b'<') {
        return true;
    }
    if s.contains("||") || s.contains('*') {
        return true;
    }
    // Hyphen range: "1.0.0 - 2.0.0"
    s.contains(" - ")
}

pub fn dependencies_string_map(v: &Value) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    let Some(obj) = v.get("dependencies").and_then(|x| x.as_object()) else {
        return m;
    };
    for (k, val) in obj {
        if let Some(s) = val.as_str() {
            m.insert(k.clone(), s.to_string());
        }
    }
    m
}

pub fn upsert_scoped_registry(
    manifest_v: &mut Value,
    name: &str,
    url: &str,
    scope: &str,
) -> Result<()> {
    let root = manifest_v
        .as_object_mut()
        .context("manifest root must be a JSON object")?;
    let arr = root
        .entry("scopedRegistries")
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .context("scopedRegistries must be an array")?;

    // 已有相同 URL 的条目 → 只追加 scope（如果还没有）
    for entry in arr.iter_mut() {
        if entry.get("url").and_then(|u| u.as_str()) == Some(url) {
            let scopes = entry
                .get_mut("scopes")
                .and_then(|s| s.as_array_mut())
                .context("scopedRegistries[].scopes must be an array")?;
            if !scopes.iter().any(|s| s.as_str() == Some(scope)) {
                scopes.push(json!(scope));
            }
            return Ok(());
        }
    }

    // 没有 → 新增条目
    arr.push(json!({
        "name": name,
        "url": url,
        "scopes": [scope]
    }));
    Ok(())
}

pub fn scoped_registries_from_value(v: &Value) -> Vec<ScopedRegistry> {
    let Some(arr) = v.get("scopedRegistries").and_then(|x| x.as_array()) else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|item| {
            let o = item.as_object()?;
            let name = o.get("name")?.as_str()?.to_string();
            let url = o.get("url")?.as_str()?.to_string();
            let scopes = o
                .get("scopes")?
                .as_array()?
                .iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect::<Vec<_>>();
            Some(ScopedRegistry { name, url, scopes })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::looks_like_npm_style_version_range;

    #[test]
    fn plain_unity_versions_are_not_ranges() {
        assert!(!looks_like_npm_style_version_range("1.2.3"));
        assert!(!looks_like_npm_style_version_range("1.0.0-preview.1"));
        assert!(!looks_like_npm_style_version_range("file:foo.tgz"));
        assert!(!looks_like_npm_style_version_range("https://x/y.git"));
    }

    #[test]
    fn npm_operators_are_ranges() {
        assert!(looks_like_npm_style_version_range("^1.0.0"));
        assert!(looks_like_npm_style_version_range("~1.0.0"));
        assert!(looks_like_npm_style_version_range(">=1.0.0"));
        assert!(looks_like_npm_style_version_range(">1.0.0"));
        assert!(looks_like_npm_style_version_range("1.0.0 - 2.0.0"));
        assert!(looks_like_npm_style_version_range("1.*.0"));
    }
}

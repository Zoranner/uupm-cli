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

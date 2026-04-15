use crate::manifest::{
    dependencies_string_map, load_manifest_value, looks_like_npm_style_version_range, MANIFEST_PATH,
};
use crate::output;
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::fs;
use std::path::Path;

const PACKAGES_LOCK_PATH: &str = "Packages/packages-lock.json";

/// Offline checks for `Packages/manifest.json` and related paths.
pub fn run() -> Result<()> {
    let mut errors = 0usize;
    let mut warnings = 0usize;

    if !Path::new("Packages").is_dir() {
        output::note("No ./Packages directory; use Unity project root for full checks.");
    }

    if !Path::new(MANIFEST_PATH).exists() {
        output::note(format!("No {} in this directory.", MANIFEST_PATH));
        return Ok(());
    }

    let manifest_v = load_manifest_value(MANIFEST_PATH)?;
    let deps = dependencies_string_map(&manifest_v);

    for (name, ver) in &deps {
        if looks_like_npm_style_version_range(ver) {
            output::error_line(format!(
                "dependency {:?} has {:?} (npm-style range; Unity expects plain SemVer for registry entries)",
                name, ver
            ));
            errors += 1;
        }

        if let Some(rest) = ver.strip_prefix("file:") {
            let p = Path::new("Packages").join(rest);
            if !p.exists() {
                output::error_line(format!("{:?} → missing path {}", name, p.display()));
                errors += 1;
            }
        }
    }

    if Path::new(PACKAGES_LOCK_PATH).is_file() {
        let raw = fs::read_to_string(PACKAGES_LOCK_PATH)
            .with_context(|| format!("read {}", PACKAGES_LOCK_PATH))?;
        match serde_json::from_str::<Value>(&raw) {
            Ok(lock) => {
                if let Some(lock_obj) = lock.as_object() {
                    for msg in lock_check_messages(&deps, lock_obj) {
                        output::warning(msg);
                        warnings += 1;
                    }
                } else {
                    output::error_line(format!(
                        "{} root must be a JSON object",
                        PACKAGES_LOCK_PATH
                    ));
                    errors += 1;
                }
            }
            Err(e) => {
                output::error_line(format!("invalid JSON in {}: {}", PACKAGES_LOCK_PATH, e));
                errors += 1;
            }
        }
    }

    if errors == 0 {
        if warnings == 0 {
            output::success(format!(
                "{} — {} direct dependencies, no issues reported.",
                MANIFEST_PATH,
                deps.len()
            ));
        } else {
            output::warning(format!(
                "{} — {} direct dependencies, {} warning(s) above.",
                MANIFEST_PATH,
                deps.len(),
                warnings
            ));
        }
        Ok(())
    } else {
        bail!("doctor: {} error(s) above", errors);
    }
}

/// Warnings that would be printed for registry direct deps vs `packages-lock.json`.
fn lock_check_messages(
    deps: &std::collections::BTreeMap<String, String>,
    lock_obj: &serde_json::Map<String, Value>,
) -> Vec<String> {
    let mut out = Vec::new();
    for (name, ver) in deps {
        if looks_like_npm_style_version_range(ver) {
            continue;
        }
        if ver.starts_with("file:") || ver.starts_with("git:") || ver.starts_with("https://") {
            continue;
        }

        let Some(entry) = lock_obj.get(name) else {
            out.push(format!(
                "{:?} is in manifest but not in {} (re-resolve in Unity Editor)",
                name, PACKAGES_LOCK_PATH
            ));
            continue;
        };
        let Some(lock_ver) = entry.get("version").and_then(|v| v.as_str()) else {
            continue;
        };
        if lock_ver != ver.as_str() {
            out.push(format!(
                "{:?} manifest version {:?} differs from lock {:?}",
                name, ver, lock_ver
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::lock_check_messages;
    use serde_json::{json, Map, Value};
    use std::collections::BTreeMap;

    #[test]
    fn lock_check_skips_non_registry_entries() {
        let mut deps = BTreeMap::new();
        deps.insert("a".to_string(), "file:foo.tgz".to_string());
        deps.insert("b".to_string(), "git:https://x".to_string());
        deps.insert("c".to_string(), "https://x/y.git".to_string());
        deps.insert("d".to_string(), "^1.0.0".to_string());
        let lock = Map::new();
        assert!(lock_check_messages(&deps, &lock).is_empty());
    }

    #[test]
    fn lock_check_warns_missing_package() {
        let mut deps = BTreeMap::new();
        deps.insert("com.vendor.pkg".to_string(), "1.0.0".to_string());
        let lock = Map::new();
        let msgs = lock_check_messages(&deps, &lock);
        assert_eq!(msgs.len(), 1);
        assert!(msgs[0].contains("com.vendor.pkg"));
        assert!(msgs[0].contains("packages-lock.json"));
    }

    #[test]
    fn lock_check_warns_version_mismatch() {
        let mut deps = BTreeMap::new();
        deps.insert("com.vendor.pkg".to_string(), "2.0.0".to_string());
        let mut lock = Map::new();
        lock.insert("com.vendor.pkg".to_string(), json!({ "version": "1.0.0" }));
        let msgs = lock_check_messages(&deps, &lock);
        assert_eq!(msgs.len(), 1);
        assert!(msgs[0].contains("differs from lock"));
    }

    #[test]
    fn lock_check_quiet_when_version_matches() {
        let mut deps = BTreeMap::new();
        deps.insert("com.vendor.pkg".to_string(), "1.0.0".to_string());
        let mut lock = Map::new();
        lock.insert("com.vendor.pkg".to_string(), json!({ "version": "1.0.0" }));
        assert!(lock_check_messages(&deps, &lock).is_empty());
    }

    #[test]
    fn lock_check_ignores_entry_without_version_field() {
        let mut deps = BTreeMap::new();
        deps.insert("com.vendor.pkg".to_string(), "1.0.0".to_string());
        let mut lock = Map::new();
        lock.insert("com.vendor.pkg".to_string(), Value::Object(Map::new()));
        assert!(lock_check_messages(&deps, &lock).is_empty());
    }
}

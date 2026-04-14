use crate::manifest::{
    dependencies_string_map, load_manifest_value, looks_like_npm_style_version_range, MANIFEST_PATH,
};
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
        println!("Note: ./Packages not found; use Unity project root for full checks.");
    }

    if !Path::new(MANIFEST_PATH).exists() {
        println!("No {} in this directory.", MANIFEST_PATH);
        return Ok(());
    }

    let manifest_v = load_manifest_value(MANIFEST_PATH)?;
    let deps = dependencies_string_map(&manifest_v);

    for (name, ver) in &deps {
        if looks_like_npm_style_version_range(ver) {
            println!(
                "Error: dependency {:?} has {:?} (npm-style range; Unity expects plain SemVer for registry entries)",
                name, ver
            );
            errors += 1;
        }

        if let Some(rest) = ver.strip_prefix("file:") {
            let p = Path::new("Packages").join(rest);
            if !p.exists() {
                println!("Error: {:?} → missing path {}", name, p.display());
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
                    check_lock_against_manifest(&deps, lock_obj, &mut warnings);
                } else {
                    println!("Error: {} root must be a JSON object", PACKAGES_LOCK_PATH);
                    errors += 1;
                }
            }
            Err(e) => {
                println!("Error: invalid JSON in {}: {}", PACKAGES_LOCK_PATH, e);
                errors += 1;
            }
        }
    }

    if errors == 0 {
        if warnings == 0 {
            println!(
                "OK: {} — {} direct dependencies, no issues reported.",
                MANIFEST_PATH,
                deps.len()
            );
        } else {
            println!(
                "OK: {} — {} direct dependencies, {} warning(s) above.",
                MANIFEST_PATH,
                deps.len(),
                warnings
            );
        }
        Ok(())
    } else {
        bail!("doctor: {} error(s) above", errors);
    }
}

fn check_lock_against_manifest(
    deps: &std::collections::BTreeMap<String, String>,
    lock_obj: &serde_json::Map<String, Value>,
    warnings: &mut usize,
) {
    for (name, ver) in deps {
        if looks_like_npm_style_version_range(ver) {
            continue;
        }
        if ver.starts_with("file:") || ver.starts_with("git:") || ver.starts_with("https://") {
            continue;
        }

        let Some(entry) = lock_obj.get(name) else {
            println!(
                "Warning: {:?} is in manifest but not in {} (re-resolve in Unity Editor)",
                name, PACKAGES_LOCK_PATH
            );
            *warnings += 1;
            continue;
        };
        let Some(lock_ver) = entry.get("version").and_then(|v| v.as_str()) else {
            continue;
        };
        if lock_ver != ver.as_str() {
            println!(
                "Warning: {:?} manifest version {:?} differs from lock {:?}",
                name, ver, lock_ver
            );
            *warnings += 1;
        }
    }
}

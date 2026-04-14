use crate::manifest::{
    dependencies_string_map, load_manifest_value, looks_like_npm_style_version_range, MANIFEST_PATH,
};
use anyhow::{bail, Result};
use std::path::Path;

/// Offline checks for `Packages/manifest.json` and related paths.
pub fn run() -> Result<()> {
    let mut errors = 0usize;

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

    if errors == 0 {
        println!(
            "OK: {} — {} direct dependencies, no issues reported.",
            MANIFEST_PATH,
            deps.len()
        );
        Ok(())
    } else {
        bail!("doctor: {} error(s) above", errors);
    }
}

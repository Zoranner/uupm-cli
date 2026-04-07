use crate::config::{default_origin_registry_url, read_configs};
use crate::manifest::{
    empty_manifest_object, load_manifest_value, save_manifest_pretty, scoped_registries_from_value,
    RegistryPackageBody, ScopedRegistry, MANIFEST_PATH,
};
use crate::versions::pick_latest_stable;
use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

/// Add or update a UPM package in `Packages/manifest.json`.
/// - `embed == false`: write a semver range string (Unity resolves from the registry).
/// - `embed == true`: download `{name}-{version}.tgz` under `Packages/` and set `file:…` like `freeze`.
///
/// Accepts `com.vendor.pkg` or `com.vendor.pkg@1.2.3`.
pub async fn install_upm_package(client: &Client, spec: &str, embed: bool) -> Result<()> {
    let parts: Vec<&str> = spec.split('@').collect();
    let package_name = parts[0].trim();
    if package_name.is_empty() {
        anyhow::bail!("empty package name");
    }
    let requested = parts.get(1).map(|s| s.trim()).filter(|s| !s.is_empty());

    let configs = read_configs()?;
    let default_url = default_origin_registry_url(&configs)?;

    let mut manifest_v = if Path::new(MANIFEST_PATH).exists() {
        load_manifest_value(MANIFEST_PATH)?
    } else {
        empty_manifest_object()
    };

    let scoped = scoped_registries_from_value(&manifest_v);
    let registry_url = match_registry_url(package_name, &scoped, &default_url);

    let url = format!("{registry_url}/{package_name}");
    println!("Fetching {} …", url);
    let body: RegistryPackageBody = client
        .get(&url)
        .send()
        .await?
        .error_for_status()
        .with_context(|| format!("GET {url}"))?
        .json()
        .await
        .with_context(|| format!("parse registry JSON for {package_name}"))?;

    if body.versions.is_empty() {
        anyhow::bail!("registry returned no versions for {package_name}");
    }

    let version_keys: Vec<String> = body.versions.keys().cloned().collect();
    let chosen = if let Some(req) = requested {
        let cleaned = req.replace(['[', ']'], "");
        if !body.versions.contains_key(&cleaned) {
            anyhow::bail!("version {cleaned} not published for {package_name}");
        }
        cleaned
    } else {
        pick_latest_stable(&version_keys).with_context(|| {
            format!("could not pick a version for {package_name} (registry returned no versions)")
        })?
    };

    let version_info = body
        .versions
        .get(&chosen)
        .context("selected version missing from registry response")?;

    let dep_string = if embed {
        let tarball_name = format!("{package_name}-{chosen}.tgz");
        let download_url = &version_info.dist.tarball;
        println!("Downloading {} …", download_url);
        let bytes = client
            .get(download_url)
            .send()
            .await?
            .error_for_status()
            .with_context(|| format!("GET tarball {download_url}"))?
            .bytes()
            .await
            .with_context(|| "read tarball body")?;
        fs::create_dir_all("Packages")?;
        let tgz_path = Path::new("Packages").join(&tarball_name);
        fs::write(&tgz_path, &bytes).with_context(|| format!("write {}", tgz_path.display()))?;
        format!("file:{tarball_name}")
    } else {
        chosen.clone()
    };

    let Some(root) = manifest_v.as_object_mut() else {
        anyhow::bail!("manifest root must be a JSON object");
    };
    let deps = root.entry("dependencies").or_insert_with(|| json!({}));
    let Some(dep_obj) = deps.as_object_mut() else {
        anyhow::bail!("manifest.dependencies must be a JSON object");
    };
    dep_obj.insert(package_name.to_string(), Value::String(dep_string.clone()));

    fs::create_dir_all("Packages")?;
    save_manifest_pretty(MANIFEST_PATH, &manifest_v)?;

    if embed {
        println!(
            "Added {} → {} in {} (tarball under Packages/, registry {}).",
            package_name, dep_string, MANIFEST_PATH, registry_url
        );
    } else {
        println!(
            "Added {}@{} to {} (registry: {}).",
            package_name, chosen, MANIFEST_PATH, registry_url
        );
    }
    Ok(())
}

fn match_registry_url(package_name: &str, scoped: &[ScopedRegistry], default: &str) -> String {
    for reg in scoped {
        for scope in &reg.scopes {
            if package_name.starts_with(scope) {
                return reg.url.trim_end_matches('/').to_string();
            }
        }
    }
    default.trim_end_matches('/').to_string()
}

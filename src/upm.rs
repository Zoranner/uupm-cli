use crate::config::{origin_bearer_token, read_configs, resolve_origin_registry};
use crate::manifest::{
    empty_manifest_object, load_manifest_value, save_manifest_pretty, scoped_registries_from_value,
    upsert_scoped_registry, RegistryPackageBody, MANIFEST_PATH,
};
use crate::versions::pick_latest_stable;
use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

/// Add or update a UPM package in `Packages/manifest.json`.
/// - `embed == false`: write an **exact** registry version string (e.g. `1.2.3`), the usual form in
///   Unity `dependencies`. This CLI does not write npm-style ranges (`^`, `>=`, …).
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

    let mut manifest_v = if Path::new(MANIFEST_PATH).exists() {
        load_manifest_value(MANIFEST_PATH)?
    } else {
        empty_manifest_object()
    };

    // 1. manifest 里已有 scopedRegistries 匹配 → 直接用，不动 manifest
    let scoped = scoped_registries_from_value(&manifest_v);
    let matched = scoped.iter().find(|r| {
        r.scopes
            .iter()
            .any(|s| package_name.starts_with(s.as_str()))
    });
    let registry_url = if let Some(reg) = matched {
        reg.url.trim_end_matches('/').to_string()
    } else {
        // 2. 从 ~/.upmrc.toml 的 origin sources 里按 scope 匹配
        let (reg_name, reg_src) = resolve_origin_registry(package_name, &configs)?;
        let url = reg_src.url.trim_end_matches('/').to_string();

        // 非默认源 → 自动写入 manifest scopedRegistries
        if reg_name != configs.registry.origin.default {
            let scope = package_scope(package_name);
            upsert_scoped_registry(&mut manifest_v, reg_name, &url, &scope)?;
        }
        url
    };

    let token = origin_bearer_token(&configs, &registry_url, matched);

    let fetch_url = format!("{registry_url}/{package_name}");
    println!("Fetching {} …", fetch_url);
    let mut req = client.get(&fetch_url);
    if let Some(t) = token {
        req = req.bearer_auth(t);
    }
    let body: RegistryPackageBody = req
        .send()
        .await?
        .error_for_status()
        .with_context(|| format!("GET {fetch_url}"))?
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
        pick_latest_stable(&version_keys)
            .with_context(|| format!("could not pick a version for {package_name}"))?
    };

    let version_info = body
        .versions
        .get(&chosen)
        .context("selected version missing from registry response")?;

    let dep_string = if embed {
        let tarball_name = format!("{package_name}-{chosen}.tgz");
        let download_url = &version_info.dist.tarball;
        println!("Downloading {} …", download_url);
        let mut t_req = client.get(download_url.as_str());
        if let Some(t) = token {
            t_req = t_req.bearer_auth(t);
        }
        let bytes = t_req
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

/// 取包名前两段作为 scope，例如 `com.unity.addressables` → `com.unity`
fn package_scope(package_name: &str) -> String {
    let parts: Vec<&str> = package_name.splitn(3, '.').collect();
    if parts.len() >= 2 {
        format!("{}.{}", parts[0], parts[1])
    } else {
        package_name.to_string()
    }
}

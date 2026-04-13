use crate::config::{read_configs, resolve_origin_registry};
use crate::manifest::{
    dependencies_string_map, load_manifest_value, save_manifest_pretty,
    scoped_registries_from_value, RegistryPackageBody, MANIFEST_PATH,
};
use crate::versions::{cmp_version_strings, pick_latest_stable};
use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;
use std::path::Path;

pub async fn upgrade_packages(client: &Client, name: Option<&str>) -> Result<()> {
    if !Path::new(MANIFEST_PATH).exists() {
        anyhow::bail!("no {} found", MANIFEST_PATH);
    }
    let manifest_v = load_manifest_value(MANIFEST_PATH)?;
    let deps = dependencies_string_map(&manifest_v);

    let targets: Vec<(String, String)> = if let Some(n) = name {
        let ver = deps
            .get(n)
            .with_context(|| format!("package {:?} not found in manifest", n))?
            .clone();
        vec![(n.to_string(), ver)]
    } else {
        deps.into_iter().collect()
    };

    for (pkg_name, pkg_version) in targets {
        upgrade_one(client, &pkg_name, &pkg_version).await?;
    }
    Ok(())
}

async fn upgrade_one(client: &Client, pkg_name: &str, pkg_version: &str) -> Result<()> {
    // 跳过 file: / git: / https:
    if pkg_version.starts_with("file:")
        || pkg_version.starts_with("git:")
        || pkg_version.starts_with("https://")
    {
        println!("Skipped (local/git): {pkg_name}@{pkg_version}");
        return Ok(());
    }
    // 跳过 Unity 内置模块
    if pkg_name.starts_with("com.unity.modules") || pkg_name.starts_with("com.unity.feature") {
        println!("Skipped (builtin): {pkg_name}@{pkg_version}");
        return Ok(());
    }

    // NuGet 包走独立升级流程
    if pkg_name.starts_with("org.nuget.") {
        println!("Upgrading NuGet: {pkg_name}…");
        crate::nuget::upgrade_nuget_package(client, pkg_name, None).await?;
        return Ok(());
    }

    // UPM 包：查注册表最新版本
    let manifest_v = load_manifest_value(MANIFEST_PATH)?;
    let configs = read_configs()?;
    let scoped = scoped_registries_from_value(&manifest_v);
    let registry_url = if let Some(reg) = scoped
        .iter()
        .find(|r| r.scopes.iter().any(|s| pkg_name.starts_with(s.as_str())))
    {
        reg.url.trim_end_matches('/').to_string()
    } else {
        let (_, src) = resolve_origin_registry(pkg_name, &configs)?;
        src.url.trim_end_matches('/').to_string()
    };

    let fetch_url = format!("{registry_url}/{pkg_name}");
    let body: RegistryPackageBody = client
        .get(&fetch_url)
        .send()
        .await?
        .error_for_status()
        .with_context(|| format!("GET {fetch_url}"))?
        .json()
        .await
        .with_context(|| format!("parse registry JSON for {pkg_name}"))?;

    if body.versions.is_empty() {
        println!("Skipped (no versions): {pkg_name}");
        return Ok(());
    }

    let version_keys: Vec<String> = body.versions.keys().cloned().collect();
    let latest = pick_latest_stable(&version_keys)
        .with_context(|| format!("could not pick a version for {pkg_name}"))?;

    if cmp_version_strings(&latest, pkg_version).is_le() {
        println!("Up to date: {pkg_name}@{pkg_version}");
        return Ok(());
    }

    // 写回 manifest
    let mut manifest_v = load_manifest_value(MANIFEST_PATH)?;
    let root = manifest_v
        .as_object_mut()
        .context("manifest root must be a JSON object")?;
    let deps = root
        .get_mut("dependencies")
        .and_then(|d| d.as_object_mut())
        .context("manifest.dependencies missing")?;
    deps.insert(pkg_name.to_string(), Value::String(latest.clone()));
    save_manifest_pretty(MANIFEST_PATH, &manifest_v)?;
    println!("Upgraded: {pkg_name}  {} → {}", pkg_version, latest);
    Ok(())
}

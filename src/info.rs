use crate::config::{origin_bearer_token, read_configs, resolve_origin_registry};
use crate::manifest::{
    load_manifest_value, scoped_registries_from_value, RegistryPackageBody, MANIFEST_PATH,
};
use crate::versions::pick_latest_stable;
use anyhow::{Context, Result};
use reqwest::Client;
use reqwest::Url;
use serde_json::Value;
use std::path::Path;

/// Resolve registry base URL and optional bearer token (manifest scoped registries, then ~/.upmrc).
pub fn resolve_origin_for_package_query(
    package_name: &str,
    registry_override: Option<&str>,
) -> Result<(String, Option<String>)> {
    let configs = read_configs()?;
    if let Some(reg_name) = registry_override {
        let src = configs
            .registry
            .origin
            .sources
            .get(reg_name)
            .with_context(|| format!("unknown origin registry {:?}", reg_name))?;
        let url = src.url.trim_end_matches('/').to_string();
        return Ok((url, src.token.clone()));
    }
    if Path::new(MANIFEST_PATH).exists() {
        let manifest_v = load_manifest_value(MANIFEST_PATH)?;
        let scoped = scoped_registries_from_value(&manifest_v);
        let matched = scoped.iter().find(|r| {
            r.scopes
                .iter()
                .any(|s| package_name.starts_with(s.as_str()))
        });
        if let Some(reg) = matched {
            let url = reg.url.trim_end_matches('/').to_string();
            let token = origin_bearer_token(&configs, &url, Some(reg)).map(String::from);
            return Ok((url, token));
        }
    }
    let (_n, src) = resolve_origin_registry(package_name, &configs)?;
    let url = src.url.trim_end_matches('/').to_string();
    let token = origin_bearer_token(&configs, &url, None)
        .map(String::from)
        .or_else(|| src.token.clone());
    Ok((url, token))
}

fn resolve_default_origin_registry() -> Result<(String, Option<String>)> {
    let configs = read_configs()?;
    let default_name = configs.registry.origin.default.clone();
    let src = configs
        .registry
        .origin
        .sources
        .get(&default_name)
        .with_context(|| format!("default origin registry {:?} missing", default_name))?;
    let url = src.url.trim_end_matches('/').to_string();
    Ok((url, src.token.clone()))
}

pub async fn print_package_info(
    client: &Client,
    package_name: &str,
    registry: Option<&str>,
) -> Result<()> {
    let (registry_url, token) = resolve_origin_for_package_query(package_name, registry)?;
    let fetch_url = format!("{registry_url}/{package_name}");
    let mut req = client.get(&fetch_url);
    if let Some(t) = token.as_deref() {
        req = req.bearer_auth(t);
    }
    let body: Value = req
        .send()
        .await
        .with_context(|| format!("GET {fetch_url}"))?
        .error_for_status()
        .with_context(|| format!("registry returned an error for {package_name}"))?
        .json()
        .await
        .with_context(|| format!("parse registry JSON for {package_name}"))?;

    let name = body
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or(package_name);
    let desc = body
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    println!("Package:  {name}");
    println!("Registry: {registry_url}");
    if !desc.is_empty() {
        println!("Description: {desc}");
    }

    if let Some(dt) = body.get("dist-tags").and_then(|v| v.as_object()) {
        if let Some(latest) = dt.get("latest").and_then(|v| v.as_str()) {
            println!("dist-tags.latest: {latest}");
        }
    }

    // Prefer structured versions when the payload matches our UPM shape
    if let Ok(pkg) = serde_json::from_value::<RegistryPackageBody>(body.clone()) {
        if !pkg.versions.is_empty() {
            let keys: Vec<String> = pkg.versions.keys().cloned().collect();
            let latest_pick = pick_latest_stable(&keys).ok();
            if let Some(v) = latest_pick {
                println!("Latest (stable heuristic): {v}");
            }
            println!("Versions ({}):", keys.len());
            let show: Vec<&String> = keys.iter().rev().take(25).collect();
            for v in show.iter().rev() {
                println!("  {v}");
            }
            if keys.len() > 25 {
                println!("  … {} more", keys.len() - 25);
            }
        } else if let Some(vers) = body.get("versions").and_then(|v| v.as_object()) {
            print_version_keys(vers);
        }
    } else if let Some(vers) = body.get("versions").and_then(|v| v.as_object()) {
        print_version_keys(vers);
    }

    Ok(())
}

fn print_version_keys(vers: &serde_json::Map<String, Value>) {
    println!("Versions ({}):", vers.len());
    let mut keys: Vec<&str> = vers.keys().map(String::as_str).collect();
    keys.sort();
    for v in keys.iter().rev().take(25).rev() {
        println!("  {v}");
    }
    if keys.len() > 25 {
        println!("  … {} more", keys.len() - 25);
    }
}

pub async fn print_search_results(
    client: &Client,
    query: &str,
    registry: Option<&str>,
    limit: usize,
) -> Result<()> {
    let limit = limit.clamp(1, 250);
    let (registry_url, token) = if let Some(name) = registry {
        let configs = read_configs()?;
        let src = configs
            .registry
            .origin
            .sources
            .get(name)
            .with_context(|| format!("unknown origin registry {:?}", name))?;
        let url = src.url.trim_end_matches('/').to_string();
        (url, src.token.clone())
    } else {
        resolve_default_origin_registry()?
    };

    let base = format!("{}/-/v1/search", registry_url.trim_end_matches('/'));
    let url = Url::parse_with_params(&base, [("text", query), ("size", &limit.to_string())])
        .with_context(|| format!("build search URL for {:?}", base))?;

    let mut req = client.get(url);
    if let Some(t) = token.as_deref() {
        req = req.bearer_auth(t);
    }
    let resp = req.send().await?;
    if !resp.status().is_success() {
        println!(
            "Search is not available or failed (HTTP {}). Many private Unity registries do not implement npm's /-/v1/search.",
            resp.status().as_u16()
        );
        return Ok(());
    }
    let body: Value = resp.json().await.context("parse search JSON")?;

    let Some(objects) = body.get("objects").and_then(|v| v.as_array()) else {
        println!("Unexpected search response (no objects array).");
        return Ok(());
    };
    if objects.is_empty() {
        println!("No packages matched {:?}.", query);
        return Ok(());
    }
    println!("Registry: {registry_url}");
    for obj in objects {
        let Some(pkg) = obj.get("package") else {
            continue;
        };
        let name = pkg.get("name").and_then(|v| v.as_str()).unwrap_or("?");
        let ver = pkg
            .get("version")
            .and_then(|v| v.as_str())
            .or_else(|| {
                pkg.get("dist-tags")
                    .and_then(|d| d.get("latest"))
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("-");
        println!("{name}@{ver}");
    }
    Ok(())
}

use crate::config::read_configs;
use crate::meta::{generate_meta_files, MetaTemplateManager};
use crate::spinner::step_spinner;
use crate::versions::pick_latest_stable;
use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashSet, VecDeque};
use std::fs;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

const PACKAGES_PATH: &str = "Packages";
const UNIT_SCOPE: &str = "org.nuget";
const OFFICIAL_FLAT: &str = "https://api.nuget.org/v3-flatcontainer/";

#[derive(Clone)]
struct ParsedNuspec {
    description: String,
    tags: Vec<String>,
    license: String,
    target_framework: String,
    dependencies: Vec<(String, String)>,
    dependencies_map: BTreeMap<String, String>,
}

pub async fn install_nuget_package(
    client: &Client,
    name: &str,
    source: Option<&str>,
) -> Result<()> {
    let mut queue = VecDeque::new();
    let mut installed: HashSet<String> = HashSet::new();
    queue.push_back(name.to_string());
    let meta_manager = MetaTemplateManager::new()?;
    while let Some(spec) = queue.pop_front() {
        if installed.contains(&spec) {
            continue;
        }
        installed.insert(spec.clone());
        println!("\n> {}", spec);
        resolve_one(client, &meta_manager, &spec, source, true, &mut queue, &installed).await?;
    }
    Ok(())
}

async fn resolve_one(
    client: &Client,
    meta_manager: &MetaTemplateManager,
    spec: &str,
    source: Option<&str>,
    recurse: bool,
    queue: &mut VecDeque<String>,
    installed: &HashSet<String>,
) -> Result<()> {
    let parts: Vec<&str> = spec.split('@').collect();
    let pascal_name = parts[0].trim();
    if pascal_name.is_empty() {
        return Err(anyhow!("empty package name"));
    }
    let kebab_name = pascal_name.to_lowercase();
    let requested_version = parts.get(1).map(|s| s.trim()).filter(|s| !s.is_empty());

    let nuget_pkg_path = Path::new(PACKAGES_PATH).join(format!("{pascal_name}.nupkg"));
    let unity_pkg_name = format!("{UNIT_SCOPE}.{kebab_name}");

    let nuget_base_url = get_nuget_base_url(client, source).await?;
    let index_url = format!("{nuget_base_url}{kebab_name}/index.json");
    let v: Value = client
        .get(&index_url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let versions = v
        .get("versions")
        .and_then(|x| x.as_array())
        .context("index.json missing versions")?;
    let version_strings: Vec<String> = versions
        .iter()
        .filter_map(|x| x.as_str().map(String::from))
        .collect();

    let target_version = if let Some(ver) = requested_version {
        let cleaned = ver.replace(['[', ']'], "");
        if !version_strings.iter().any(|v| v == &cleaned) {
            return Err(anyhow!(
                "The version {cleaned} of {pascal_name} does not exist."
            ));
        }
        cleaned
    } else {
        pick_latest_stable(&version_strings)?
    };

    println!("Found: {pascal_name}@{target_version}");

    let unity_pkg_path =
        Path::new(PACKAGES_PATH).join(format!("{unity_pkg_name}-{target_version}"));

    step_spinner("Downloading nuget package...", async {
        download_nupkg(client, &nuget_base_url, &kebab_name, &target_version, &nuget_pkg_path).await
    })
    .await?;

    let nuspec_xml = read_nuspec_from_nupkg(&nuget_pkg_path, &kebab_name)?;
    let parsed = parse_nuspec(&nuspec_xml, &kebab_name)?;

    if recurse {
        for (id, ver) in &parsed.dependencies {
            let dep_spec = format!("{id}@{ver}");
            if !installed.contains(&dep_spec) && !queue.iter().any(|x| x == &dep_spec) {
                queue.push_back(dep_spec);
            }
        }
    }

    step_spinner("Converting package info...", async {
        convert_package_info(&unity_pkg_path, &unity_pkg_name, pascal_name, &target_version, &parsed)
    })
    .await?;

    step_spinner("Extracting package to local...", async {
        extract_specific_files(&nuget_pkg_path, &unity_pkg_path, &parsed.target_framework)?;
        let _ = fs::remove_file(&nuget_pkg_path);
        Ok(format!("Extracted to {unity_pkg_name}@{target_version}."))
    })
    .await?;

    step_spinner("Generating meta files...", async {
        generate_meta_files(meta_manager, &unity_pkg_name, &unity_pkg_path)?;
        Ok(format!("Generated meta files for {unity_pkg_name}."))
    })
    .await?;

    Ok(())
}

async fn download_nupkg(
    client: &Client,
    base_url: &str,
    kebab_name: &str,
    version: &str,
    dest: &Path,
) -> Result<String> {
    let url = format!("{base_url}{kebab_name}/{version}/{kebab_name}.{version}.nupkg");
    let bytes = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    fs::create_dir_all(PACKAGES_PATH)?;
    fs::write(dest, &bytes)?;
    Ok(format!("Download {kebab_name}@{version} complete."))
}

fn convert_package_info(
    unity_pkg_path: &Path,
    unity_pkg_name: &str,
    pascal_name: &str,
    version: &str,
    parsed: &ParsedNuspec,
) -> Result<String> {
    if unity_pkg_path.exists() {
        fs::remove_dir_all(unity_pkg_path)?;
    }
    fs::create_dir_all(unity_pkg_path)?;
    write_package_json(unity_pkg_path, unity_pkg_name, pascal_name, version, parsed)?;
    Ok("Converted package info to package.json.".to_string())
}

async fn get_nuget_base_url(client: &Client, source: Option<&str>) -> Result<String> {
    let cfg = read_configs()?;
    let nuget_cfg = &cfg.registry.nuget;
    let source_name = source.unwrap_or(&nuget_cfg.default);
    let Some(src) = nuget_cfg.sources.get(source_name) else {
        return Err(anyhow!("unknown nuget source {:?}", source_name));
    };
    let index_url = &src.url;
    let response = client.get(index_url).send().await;
    let Ok(resp) = response else {
        return Ok(OFFICIAL_FLAT.to_string());
    };
    let Ok(resp) = resp.error_for_status() else {
        return Ok(OFFICIAL_FLAT.to_string());
    };
    let Ok(body) = resp.json::<Value>().await else {
        return Ok(OFFICIAL_FLAT.to_string());
    };
    if let Some(resources) = body.get("resources").and_then(|r| r.as_array()) {
        for r in resources {
            if r.get("@type").and_then(|t| t.as_str()) == Some("PackageBaseAddress/3.0.0") {
                if let Some(id) = r.get("@id").and_then(|x| x.as_str()) {
                    let mut base = id.to_string();
                    if !base.ends_with('/') {
                        base.push('/');
                    }
                    return Ok(base);
                }
            }
        }
    }
    Ok(OFFICIAL_FLAT.to_string())
}

fn read_nuspec_from_nupkg(nupkg_path: &Path, kebab_name: &str) -> Result<String> {
    let file = fs::File::open(nupkg_path)?;
    let mut archive = ZipArchive::new(file)?;
    let expected = format!("{kebab_name}.nuspec").to_lowercase();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_lowercase();
        if name == expected || name.ends_with(&format!("/{expected}")) {
            let mut s = String::new();
            entry.read_to_string(&mut s)?;
            return Ok(s);
        }
    }
    Err(anyhow!("nuspec {expected} not found in nupkg"))
}

fn parse_nuspec(xml: &str, _kebab_name: &str) -> Result<ParsedNuspec> {
    let doc = roxmltree::Document::parse(xml).context("parse nuspec xml")?;
    let mut description = String::new();
    let mut tags: Vec<String> = Vec::new();
    let mut license = "Unknown".to_string();
    let mut groups: Vec<(String, Vec<(String, String)>)> = Vec::new();

    for node in doc.descendants().filter(|n| n.is_element()) {
        match node.tag_name().name() {
            "description" if node.parent().map(|p| p.tag_name().name()) == Some("metadata") => {
                description = node.text().unwrap_or("").trim().to_string();
            }
            "tags" if node.parent().map(|p| p.tag_name().name()) == Some("metadata") => {
                if let Some(t) = node.text() {
                    tags = t.split_whitespace().map(String::from).collect();
                }
            }
            "license" if node.parent().map(|p| p.tag_name().name()) == Some("metadata") => {
                if let Some(t) = node.text() {
                    license = t.trim().to_string();
                } else if let Some(ty) = node.attribute("type") {
                    license = ty.to_string();
                }
            }
            "group" => {
                let parent = node.parent().map(|p| p.tag_name().name());
                if parent != Some("dependencies") {
                    continue;
                }
                let Some(tf) = node.attribute("targetFramework") else {
                    continue;
                };
                let mut deps = Vec::new();
                for child in node.children().filter(|c| c.is_element()) {
                    if child.tag_name().name() == "dependency" {
                        if let (Some(id), Some(ver)) =
                            (child.attribute("id"), child.attribute("version"))
                        {
                            deps.push((id.to_string(), ver.to_string()));
                        }
                    }
                }
                groups.push((tf.to_string(), deps));
            }
            _ => {}
        }
    }

    let mut netstd: Vec<(String, Vec<(String, String)>)> = groups
        .into_iter()
        .filter(|(tf, _)| tf.starts_with(".NETStandard"))
        .collect();
    if netstd.is_empty() {
        return Err(anyhow!("The library does not support Unity."));
    }
    netstd.sort_by(|a, b| b.0.cmp(&a.0));
    let (target_framework, dependencies) = netstd.into_iter().next().expect("netstd non-empty");

    let mut dependencies_map = BTreeMap::new();
    for (id, ver) in &dependencies {
        let key = format!("{UNIT_SCOPE}.{}", id.to_lowercase());
        dependencies_map.insert(key, ver.clone());
    }

    Ok(ParsedNuspec {
        description,
        tags,
        license,
        target_framework,
        dependencies,
        dependencies_map,
    })
}

fn write_package_json(
    dir: &Path,
    unity_pkg_name: &str,
    pascal_name: &str,
    version: &str,
    parsed: &ParsedNuspec,
) -> Result<()> {
    let mut deps = Map::new();
    for (k, v) in &parsed.dependencies_map {
        deps.insert(k.clone(), json!(v));
    }
    let package = json!({
        "name": unity_pkg_name,
        "displayName": pascal_name,
        "version": version,
        "unity": "2021.3",
        "author": {
            "name": "NuGet"
        },
        "description": parsed.description,
        "type": "library",
        "keywords": parsed.tags,
        "license": parsed.license,
        "dependencies": deps,
        "repository": {}
    });
    let path = dir.join("package.json");
    fs::write(path, serde_json::to_string_pretty(&package)?)?;
    Ok(())
}

fn extract_specific_files(
    nupkg_path: &Path,
    unity_pkg_path: &Path,
    target_framework: &str,
) -> Result<()> {
    let low = target_framework.to_lowercase();
    let tail = low.strip_prefix('.').unwrap_or(low.as_str());
    let lib_prefix = format!("lib/{tail}/");
    let file = fs::File::open(nupkg_path)?;
    let mut archive = ZipArchive::new(file)?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        if entry.is_dir() {
            continue;
        }
        let is_root_file = !name.contains('/') && name != "[Content_Types].xml";
        let is_lib = name.starts_with(&lib_prefix);
        let is_runtimes = name.starts_with("runtimes/");
        if !(is_root_file || is_lib || is_runtimes) {
            continue;
        }
        let full_path = unity_pkg_path.join(&name);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = fs::File::create(&full_path)?;
        std::io::copy(&mut entry, &mut out)?;
    }
    Ok(())
}

use crate::config::{default_origin_registry_url, read_configs};
use crate::manifest::{
    dependencies_string_map, load_manifest_value, save_manifest_pretty,
    scoped_registries_from_value, RegistryPackageBody, ScopedRegistry, MANIFEST_PATH,
};
use crate::spinner::{step_spinner, SpinnerSuccess};
use anyhow::{Context, Result};
use dialoguer::{theme::ColorfulTheme, Select};
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const BUILD_IN_PACKAGES: &[&str] = &[
    "com.unity.2d.sprite",
    "com.unity.2d.tilemap",
    "com.unity.render-pipelines.core",
    "com.unity.render-pipelines.high-definition",
    "com.unity.shadergraph",
    "com.unity.rendering.denoising",
    "com.unity.ugui",
    "com.unity.render-pipelines.universal",
    "com.unity.visualeffectgraph",
];

#[derive(Default)]
pub struct FreezePatch {
    pub remove_dep: Option<String>,
    pub set_dep: Option<(String, String)>,
    pub merge_deps: BTreeMap<String, String>,
}

pub struct FreezeOutcome {
    msg: String,
    pub patch: FreezePatch,
}

impl SpinnerSuccess for FreezeOutcome {
    fn print_success(&self) {
        println!("✔ {}", self.msg);
    }
}

#[derive(Debug, Deserialize)]
struct UnityBuiltinPackageJson {
    #[serde(default)]
    dependencies: BTreeMap<String, String>,
}

pub async fn freeze_packages(client: &Client) -> Result<()> {
    let configs = read_configs()?;
    let default_registry_url = default_origin_registry_url(&configs)?;
    let editor_path = select_unity_version(&configs)?;

    if !Path::new(MANIFEST_PATH).exists() {
        println!("No {MANIFEST_PATH} file exists in the current directory.");
        return Ok(());
    }
    let mut manifest_v = load_manifest_value(MANIFEST_PATH)?;

    let mut package_map = dependencies_string_map(&manifest_v);
    let scoped = scoped_registries_from_value(&manifest_v);

    while !package_map.is_empty() {
        let (package_name, package_version) = package_map
            .iter()
            .next()
            .map(|(k, v)| (k.clone(), v.clone()))
            .unwrap();
        package_map.remove(&package_name);

        let registry_url = match_registry_url(&package_name, &scoped, &default_registry_url);

        let ep = editor_path.clone();
        let pn = package_name.clone();
        let pv = package_version.clone();
        let ru = registry_url.clone();
        let c = client.clone();
        let outcome = step_spinner("Freezing unity package...", async move {
            freeze_single(&c, &ep, &pn, &pv, &ru).await
        })
        .await?;
        apply_patch(&mut manifest_v, &mut package_map, outcome.patch);
    }

    if Path::new(MANIFEST_PATH).exists() {
        fs::copy(MANIFEST_PATH, "Packages/manifest.src.json")?;
        fs::remove_file(MANIFEST_PATH)?;
    }
    save_manifest_pretty(MANIFEST_PATH, &manifest_v)?;
    Ok(())
}

fn apply_patch(
    manifest: &mut Value,
    package_map: &mut BTreeMap<String, String>,
    patch: FreezePatch,
) {
    let Some(root) = manifest.as_object_mut() else {
        return;
    };
    let deps = root.entry("dependencies").or_insert_with(|| json!({}));
    let Some(dep_obj) = deps.as_object_mut() else {
        return;
    };
    if let Some(name) = patch.remove_dep {
        dep_obj.remove(&name);
    }
    if let Some((k, v)) = patch.set_dep {
        dep_obj.insert(k, Value::String(v));
    }
    merge_dependencies(package_map, &patch.merge_deps);
}

fn select_unity_version(configs: &crate::config::GlobalConfig) -> Result<String> {
    let keys: Vec<String> = configs.editor.version.keys().cloned().collect();
    if keys.is_empty() {
        anyhow::bail!("no Unity editors in ~/.upmrc; run `uupm editor scan` or `uupm editor add`");
    }
    let labels: Vec<&str> = keys.iter().map(String::as_str).collect();
    let sel = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Please select the Unity version:")
        .items(&labels)
        .default(0)
        .interact()?;
    configs
        .editor
        .version
        .get(&keys[sel])
        .cloned()
        .context("selected editor path missing")
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

async fn freeze_single(
    client: &Client,
    editor_path: &str,
    package_name: &str,
    package_version: &str,
    registry_url: &str,
) -> Result<FreezeOutcome> {
    if package_name.starts_with("com.unity.modules")
        || package_name.starts_with("com.unity.feature")
    {
        return Ok(FreezeOutcome {
            msg: format!("Skipped: {package_name}@{package_version}."),
            patch: FreezePatch::default(),
        });
    }
    if package_version.starts_with("file:") || package_version.starts_with("git:") {
        return Ok(FreezeOutcome {
            msg: format!("Skipped: {package_name}@{package_version}."),
            patch: FreezePatch::default(),
        });
    }

    if BUILD_IN_PACKAGES.contains(&package_name) {
        return freeze_builtin(editor_path, package_name, package_version);
    }

    freeze_from_registry(client, package_name, package_version, registry_url).await
}

fn freeze_builtin(
    editor_path: &str,
    package_name: &str,
    package_version: &str,
) -> Result<FreezeOutcome> {
    let package_path = PathBuf::from(editor_path)
        .join("Data")
        .join("Resources")
        .join("PackageManager")
        .join("BuiltInPackages")
        .join(package_name);
    let package_file = package_path.join("package.json");
    if !package_file.is_file() {
        return Ok(FreezeOutcome {
            msg: format!("Skipped: {package_name}@{package_version}."),
            patch: FreezePatch::default(),
        });
    }
    let package_content = fs::read_to_string(&package_file)?;
    let pkg: UnityBuiltinPackageJson =
        serde_json::from_str(&package_content).unwrap_or(UnityBuiltinPackageJson {
            dependencies: BTreeMap::new(),
        });
    let dest = Path::new("Packages").join(format!("{package_name}-{package_version}"));
    copy_dir_all(&package_path, &dest)?;
    let patch = FreezePatch {
        remove_dep: Some(package_name.to_string()),
        merge_deps: pkg.dependencies,
        ..Default::default()
    };
    Ok(FreezeOutcome {
        msg: format!("Frozen: {package_name}@{package_version}."),
        patch,
    })
}

async fn freeze_from_registry(
    client: &Client,
    package_name: &str,
    package_version: &str,
    registry_url: &str,
) -> Result<FreezeOutcome> {
    let package_info_url = format!("{registry_url}/{package_name}");
    let response = client.get(&package_info_url).send().await;
    let Ok(resp) = response else {
        return Ok(FreezeOutcome {
            msg: format!("Skipped: {package_name}@{package_version}."),
            patch: FreezePatch::default(),
        });
    };
    if !resp.status().is_success() {
        return Ok(FreezeOutcome {
            msg: format!("Skipped: {package_name}@{package_version}."),
            patch: FreezePatch::default(),
        });
    }
    let body: RegistryPackageBody = match resp.json().await {
        Ok(b) => b,
        Err(_) => {
            return Ok(FreezeOutcome {
                msg: format!("Skipped: {package_name}@{package_version}."),
                patch: FreezePatch::default(),
            });
        }
    };
    let Some(version_info) = body.versions.get(package_version) else {
        return Ok(FreezeOutcome {
            msg: format!("Skipped: {package_name}@{package_version}."),
            patch: FreezePatch::default(),
        });
    };
    let tarball_name = format!("{package_name}-{package_version}.tgz");
    let download_url = &version_info.dist.tarball;
    let bytes = match client.get(download_url).send().await {
        Ok(r) => match r.error_for_status() {
            Ok(r) => match r.bytes().await {
                Ok(b) => b,
                Err(_) => {
                    return Ok(FreezeOutcome {
                        msg: format!("Skipped: {package_name}@{package_version}."),
                        patch: FreezePatch::default(),
                    });
                }
            },
            Err(_) => {
                return Ok(FreezeOutcome {
                    msg: format!("Skipped: {package_name}@{package_version}."),
                    patch: FreezePatch::default(),
                });
            }
        },
        Err(_) => {
            return Ok(FreezeOutcome {
                msg: format!("Skipped: {package_name}@{package_version}."),
                patch: FreezePatch::default(),
            });
        }
    };
    fs::create_dir_all("Packages")?;
    let file_path = Path::new("Packages").join(&tarball_name);
    fs::write(&file_path, &bytes)?;
    let patch = FreezePatch {
        set_dep: Some((package_name.to_string(), format!("file:{tarball_name}"))),
        merge_deps: version_info.dependencies.clone(),
        ..Default::default()
    };
    Ok(FreezeOutcome {
        msg: format!("Frozen: {package_name}@{package_version}."),
        patch,
    })
}

fn merge_dependencies(
    package_map: &mut BTreeMap<String, String>,
    dependencies: &BTreeMap<String, String>,
) {
    for (name, ver) in dependencies {
        add_to_package_map(package_map, name, ver);
    }
}

fn add_to_package_map(map: &mut BTreeMap<String, String>, name: &str, version: &str) {
    if let Some(existing) = map.get(name) {
        if compare_versions(version, existing) > 0 {
            map.insert(name.to_string(), version.to_string());
        }
    } else {
        map.insert(name.to_string(), version.to_string());
    }
}

fn compare_versions(v1: &str, v2: &str) -> i32 {
    match crate::versions::cmp_version_strings(v1, v2) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for e in fs::read_dir(src)? {
        let e = e?;
        let ty = e.file_type()?;
        let s = e.path();
        let d = dst.join(e.file_name());
        if ty.is_dir() {
            copy_dir_all(&s, &d)?;
        } else {
            fs::copy(&s, &d)?;
        }
    }
    Ok(())
}

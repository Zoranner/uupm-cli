use crate::config::{origin_bearer_token, read_configs, resolve_origin_registry};
use crate::manifest::{
    dependencies_string_map, load_manifest_value, save_manifest_pretty,
    scoped_registries_from_value, RegistryPackageBody, MANIFEST_PATH,
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
    let editor_path = select_unity_version(&configs)?;

    if !Path::new(MANIFEST_PATH).exists() {
        println!("No {MANIFEST_PATH} file exists in the current directory.");
        return Ok(());
    }
    let mut manifest_v = load_manifest_value(MANIFEST_PATH)?;

    let mut package_map = dependencies_string_map(&manifest_v);
    let scoped = scoped_registries_from_value(&manifest_v);

    while let Some((package_name, package_version)) = package_map.pop_first() {
        // manifest scopedRegistries 优先，fallback 到 ~/.upmrc.toml
        let matched = scoped.iter().find(|r| {
            r.scopes
                .iter()
                .any(|s| package_name.starts_with(s.as_str()))
        });
        let registry_url = if let Some(reg) = matched {
            reg.url.trim_end_matches('/').to_string()
        } else {
            let (_, src) = resolve_origin_registry(&package_name, &configs)?;
            src.url.trim_end_matches('/').to_string()
        };
        let token = origin_bearer_token(&configs, &registry_url, matched).map(String::from);
        let c = client.clone();
        let ep = editor_path.clone();
        let outcome = step_spinner("Freezing unity package...", async move {
            freeze_single(
                &c,
                &ep,
                &package_name,
                &package_version,
                &registry_url,
                token,
            )
            .await
        })
        .await?;
        apply_patch(&mut manifest_v, &mut package_map, outcome.patch);
    }

    // 先写到临时文件，再备份原始清单，最后 rename，避免中途崩溃丢失数据
    let tmp_path = "Packages/manifest.tmp.json";
    save_manifest_pretty(tmp_path, &manifest_v)?;
    if Path::new(MANIFEST_PATH).exists() {
        fs::copy(MANIFEST_PATH, "Packages/manifest.src.json")?;
    }
    fs::rename(tmp_path, MANIFEST_PATH)?;
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
    let keys: Vec<String> = configs.editor.versions.keys().cloned().collect();
    if keys.is_empty() {
        anyhow::bail!(
            "no Unity editors in ~/.upmrc.toml; run `uupm editor scan` or `uupm editor add`"
        );
    }
    // 只有一个版本时直接使用，无需交互
    if keys.len() == 1 {
        return configs
            .editor
            .versions
            .get(&keys[0])
            .cloned()
            .context("selected editor path missing");
    }
    let labels: Vec<&str> = keys.iter().map(String::as_str).collect();
    let sel = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Please select the Unity version:")
        .items(&labels)
        .default(0)
        .interact()?;
    configs
        .editor
        .versions
        .get(&keys[sel])
        .cloned()
        .context("selected editor path missing")
}

async fn freeze_single(
    client: &Client,
    editor_path: &str,
    package_name: &str,
    package_version: &str,
    registry_url: &str,
    registry_token: Option<String>,
) -> Result<FreezeOutcome> {
    if package_name.starts_with("com.unity.modules")
        || package_name.starts_with("com.unity.feature")
    {
        return Ok(FreezeOutcome {
            msg: format!("Skipped: {package_name}@{package_version}."),
            patch: FreezePatch::default(),
        });
    }
    if package_version.starts_with("file:")
        || package_version.starts_with("git:")
        || package_version.starts_with("https://")
    {
        return Ok(FreezeOutcome {
            msg: format!("Skipped: {package_name}@{package_version}."),
            patch: FreezePatch::default(),
        });
    }

    if crate::manifest::looks_like_npm_style_version_range(package_version) {
        anyhow::bail!(
            "cannot freeze {}: version {:?} looks like an npm-style range; Unity expects plain SemVer for registry packages (see Unity Manual: Project manifest / Package manifest)",
            package_name,
            package_version
        );
    }

    if BUILD_IN_PACKAGES.contains(&package_name) {
        return freeze_builtin(editor_path, package_name, package_version);
    }

    freeze_from_registry(
        client,
        package_name,
        package_version,
        registry_url,
        registry_token.as_deref(),
    )
    .await
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
    let pkg: UnityBuiltinPackageJson = serde_json::from_str(&package_content)
        .with_context(|| format!("failed to parse {}", package_file.display()))?;
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
    token: Option<&str>,
) -> Result<FreezeOutcome> {
    let package_info_url = format!("{registry_url}/{package_name}");
    let mut meta_req = client.get(&package_info_url);
    if let Some(t) = token {
        meta_req = meta_req.bearer_auth(t);
    }
    let resp = meta_req
        .send()
        .await
        .with_context(|| format!("network error fetching {package_name}"))?
        .error_for_status()
        .with_context(|| format!("registry returned error for {package_name}"))?;
    let body: RegistryPackageBody = resp
        .json()
        .await
        .with_context(|| format!("failed to parse registry response for {package_name}"))?;
    let Some(version_info) = body.versions.get(package_version) else {
        anyhow::bail!("version {package_version} not found in registry for {package_name}");
    };
    let tarball_name = format!("{package_name}-{package_version}.tgz");
    let download_url = &version_info.dist.tarball;
    let mut dl_req = client.get(download_url.as_str());
    if let Some(t) = token {
        dl_req = dl_req.bearer_auth(t);
    }
    let bytes = dl_req
        .send()
        .await
        .with_context(|| format!("network error downloading tarball for {package_name}"))?
        .error_for_status()
        .with_context(|| format!("tarball download failed for {package_name}"))?
        .bytes()
        .await
        .with_context(|| format!("failed to read tarball bytes for {package_name}"))?;
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
        if crate::versions::cmp_version_strings(version, existing).is_gt() {
            map.insert(name.to_string(), version.to_string());
        }
    } else {
        map.insert(name.to_string(), version.to_string());
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

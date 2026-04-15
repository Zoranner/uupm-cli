use crate::manifest::{load_manifest_value, save_manifest_pretty, MANIFEST_PATH};
use crate::output;
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;

pub fn remove_package(name: &str) -> Result<()> {
    if !Path::new(MANIFEST_PATH).exists() {
        bail!("no {} found", MANIFEST_PATH);
    }
    let mut manifest_v = load_manifest_value(MANIFEST_PATH)?;

    let root = manifest_v
        .as_object_mut()
        .context("manifest root must be a JSON object")?;
    let deps = root
        .get_mut("dependencies")
        .and_then(|d| d.as_object_mut())
        .context("manifest.dependencies missing or not an object")?;

    let Some(val) = deps.remove(name) else {
        bail!("package {:?} not found in manifest", name);
    };

    // 清理本地文件
    if let Some(file_ref) = val.as_str().and_then(|s| s.strip_prefix("file:")) {
        // file:xxx.tgz 或 file:some-dir
        let artifact = Path::new("Packages").join(file_ref);
        if artifact.is_file() {
            fs::remove_file(&artifact).with_context(|| format!("remove {}", artifact.display()))?;
            output::note(format!("Removed file: {}", artifact.display()));
        } else if artifact.is_dir() {
            fs::remove_dir_all(&artifact)
                .with_context(|| format!("remove dir {}", artifact.display()))?;
            output::note(format!("Removed dir: {}", artifact.display()));
        }
        // 同时删除对应 .meta（如果存在）
        let meta = Path::new("Packages").join(format!("{file_ref}.meta"));
        if meta.exists() {
            fs::remove_file(&meta).with_context(|| format!("remove {}", meta.display()))?;
        }
    }

    // NuGet 包目录：org.nuget.<name>-<version>（无 file: 前缀，Unity 本地扫描）
    if name.starts_with("org.nuget.") {
        let packages_dir = Path::new("Packages");
        if packages_dir.is_dir() {
            let prefix = format!("{name}-");
            for entry in fs::read_dir(packages_dir)? {
                let entry = entry?;
                let fname = entry.file_name();
                let fname_str = fname.to_string_lossy();
                if fname_str.starts_with(&prefix) && entry.path().is_dir() {
                    fs::remove_dir_all(entry.path())
                        .with_context(|| format!("remove dir {}", entry.path().display()))?;
                    output::note(format!("Removed dir: {}", entry.path().display()));
                    // .meta
                    let meta = packages_dir.join(format!("{fname_str}.meta"));
                    if meta.exists() {
                        fs::remove_file(&meta)?;
                    }
                }
            }
        }
    }

    save_manifest_pretty(MANIFEST_PATH, &manifest_v)?;
    output::success(format!("Removed {name} from manifest."));
    Ok(())
}

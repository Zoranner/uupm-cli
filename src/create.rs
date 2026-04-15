use crate::output;
use anyhow::{bail, Result};
use std::fs;
use std::path::Path;

pub fn create_package(
    name: &str,
    display_name: Option<&str>,
    author: Option<&str>,
    version: &str,
) -> Result<()> {
    if !name.contains('.') {
        bail!("package name must be a reverse-domain identifier, e.g. com.vendor.mylib");
    }

    let dir = Path::new(name);
    if dir.exists() {
        bail!("directory {:?} already exists", name);
    }

    let display = display_name
        .map(str::to_string)
        .unwrap_or_else(|| derive_display_name(name));
    let asmdef_name = derive_asmdef_name(name);
    let author_name = author.unwrap_or("Unknown");

    // 目录结构
    let scripts_dir = dir.join("Scripts");
    fs::create_dir_all(&scripts_dir)?;

    // package.json
    let package_json = format!(
        r#"{{
    "name": "{name}",
    "displayName": "{display}",
    "description": "",
    "version": "{version}",
    "unity": "2021.3",
    "author": {{
        "name": "{author_name}"
    }},
    "license": "MIT",
    "dependencies": {{}}
}}
"#
    );
    fs::write(dir.join("package.json"), package_json)?;

    // Scripts/<AsmdefName>.asmdef
    let asmdef = format!(
        r#"{{
    "name": "{asmdef_name}",
    "references": [],
    "includePlatforms": [],
    "excludePlatforms": [],
    "allowUnsafeCode": false,
    "overrideReferences": false,
    "precompiledReferences": [],
    "autoReferenced": true,
    "defineConstraints": [],
    "versionDefines": [],
    "noEngineReferences": false
}}
"#
    );
    fs::write(scripts_dir.join(format!("{asmdef_name}.asmdef")), asmdef)?;

    // README.md
    fs::write(dir.join("README.md"), format!("# {display}\n"))?;

    output::success(format!("Created package: {name}"));
    output::item_indent(format!("{name}/"));
    output::item_indent(format!("{name}/package.json"));
    output::item_indent(format!("{name}/Scripts/{asmdef_name}.asmdef"));
    output::item_indent(format!("{name}/README.md"));
    Ok(())
}

/// `com.kimotech.template.unity-package` → `UnityPackage Template` (last segment, title-cased)
fn derive_display_name(name: &str) -> String {
    let last = name.split('.').next_back().unwrap_or(name);
    last.split('-')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// `com.kimotech.template.unity-package` → `KimoTech.Template.UnityPackage`
fn derive_asmdef_name(name: &str) -> String {
    name.split('.')
        .skip(1) // drop "com"
        .map(|seg| {
            seg.split('-')
                .map(|w| {
                    let mut c = w.chars();
                    match c.next() {
                        None => String::new(),
                        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                    }
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join(".")
}

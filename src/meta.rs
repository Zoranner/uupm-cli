use anyhow::{Context, Result};
use include_dir::{include_dir, Dir};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

static TEMPLATES: Dir<'static> = include_dir!("templates");

#[derive(Clone)]
pub struct MetaTemplateManager {
    templates: HashMap<String, String>,
}

const MACHINE_X86: u16 = 0x14c;
const MACHINE_AMD64: u16 = 0x8664;
const MACHINE_ARM64: u16 = 0xaa64;

#[derive(Debug, Clone)]
struct DllInfo {
    is_dotnet: bool,
    architecture: &'static str,
}

impl MetaTemplateManager {
    pub fn new() -> Result<Self> {
        let mut templates = HashMap::new();
        for file in TEMPLATES.files() {
            let rel = file.path().to_string_lossy().replace('\\', "/");
            if !rel.ends_with(".yml") {
                continue;
            }
            let key = template_key(&rel);
            let content = file
                .contents_utf8()
                .with_context(|| format!("template not utf-8: {}", rel))?;
            templates.insert(key, content.to_string());
        }
        Ok(Self { templates })
    }

    pub fn get_template(&self, typ: &str, guid: &str) -> Result<String> {
        let raw = self
            .templates
            .get(typ)
            .or_else(|| self.templates.get("default"))
            .with_context(|| format!("no meta template for type {:?}", typ))?;
        Ok(raw.replace("${guid}", guid))
    }

    pub fn meta_type_for_path(path: &Path) -> Result<String> {
        let meta = fs::metadata(path)?;
        let file_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        if meta.is_dir() {
            return Ok("directory".to_string());
        }
        if file_name == "package.json" {
            return Ok("package".to_string());
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        Ok(match ext.as_str() {
            "dll" => {
                let info = analyze_dll(path);
                if info.is_dotnet {
                    "plugin.dotnet".to_string()
                } else {
                    format!("plugin.native.win.{}", info.architecture)
                }
            }
            "dylib" => "plugin.native.macos.dylib".to_string(),
            "so" => "plugin.native.linux.so".to_string(),
            "png" | "jpg" | "jpeg" => "texture.default".to_string(),
            "json" | "txt" | "md" | "xml" => "text".to_string(),
            _ => "default".to_string(),
        })
    }
}

fn template_key(rel: &str) -> String {
    let p = Path::new(rel);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("default");
    let Some(parent) = p.parent().filter(|x| !x.as_os_str().is_empty()) else {
        return stem.to_string();
    };
    let parts: Vec<String> = parent
        .components()
        .filter_map(|c| c.as_os_str().to_str().map(String::from))
        .collect();
    if parts.is_empty() {
        stem.to_string()
    } else {
        format!("{}.{}", parts.join("."), stem)
    }
}

fn analyze_dll(path: &Path) -> DllInfo {
    let default = DllInfo {
        is_dotnet: false,
        architecture: "default",
    };
    let Ok(buffer) = fs::read(path) else {
        return default;
    };
    // PE 头最小需要：DOS header(64) + PE offset(4) + COFF header(20) + Optional header magic(2)
    // 实际读取到 optional_header_offset + clr_header_offset + 8，保守取 256 字节
    if buffer.len() < 256 {
        return default;
    }
    if read_u16_le(&buffer, 0) != 0x5a4d {
        return default;
    }
    let pe_offset = read_u32_le(&buffer, 0x3c) as usize;
    if pe_offset >= buffer.len().saturating_sub(4) {
        return default;
    }
    if read_u32_le(&buffer, pe_offset) != 0x00004550 {
        return default;
    }
    let machine = read_u16_le(&buffer, pe_offset + 4);
    let characteristics = read_u16_le(&buffer, pe_offset + 22);
    if characteristics & 0x2000 == 0 {
        return default;
    }
    let optional_header_offset = pe_offset + 24;
    if optional_header_offset > buffer.len().saturating_sub(216) {
        return default;
    }
    let magic = read_u16_le(&buffer, optional_header_offset);
    if magic != 0x20b && magic != 0x10b {
        return default;
    }
    let is_pe32_plus = magic == 0x20b;
    let clr_header_offset = if is_pe32_plus { 224 } else { 208 };
    let clr_rva = read_u32_le(&buffer, optional_header_offset + clr_header_offset);
    let clr_size = read_u32_le(&buffer, optional_header_offset + clr_header_offset + 4);

    let architecture: &'static str = match machine {
        MACHINE_X86 => "x86",
        MACHINE_AMD64 => "x64",
        MACHINE_ARM64 => "arm64",
        _ => "default",
    };

    DllInfo {
        is_dotnet: clr_rva != 0 && clr_size > 0,
        architecture,
    }
}

fn read_u16_le(buf: &[u8], off: usize) -> u16 {
    u16::from_le_bytes(buf.get(off..off + 2).unwrap_or(&[0, 0]).try_into().unwrap())
}

fn read_u32_le(buf: &[u8], off: usize) -> u32 {
    u32::from_le_bytes(buf.get(off..off + 4).unwrap_or(&[0; 4]).try_into().unwrap())
}

pub fn deterministic_guid(package_name: &str, relative: &str) -> String {
    use md5::{Digest, Md5};
    let mut hasher = Md5::new();
    hasher.update(format!("{}:{}", package_name, relative));
    format!("{:x}", hasher.finalize())
}

pub fn generate_meta_files(
    manager: &MetaTemplateManager,
    unity_package_id: &str,
    unity_pkg_path: &Path,
) -> Result<()> {
    fn walk(
        manager: &MetaTemplateManager,
        unity_package_id: &str,
        pkg_root: &Path,
        dir: &Path,
    ) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let full = entry.path();
            let meta_path: PathBuf = format!("{}.meta", full.display()).into();
            if !meta_path.exists() {
                let rel = full.strip_prefix(pkg_root).unwrap_or(&full);
                let rel_s = rel.to_string_lossy().replace('\\', "/");
                let guid = deterministic_guid(unity_package_id, &rel_s);
                let typ = MetaTemplateManager::meta_type_for_path(&full)?;
                let content = manager.get_template(&typ, &guid)?;
                fs::write(&meta_path, content)?;
            }
            if full.is_dir() {
                walk(manager, unity_package_id, pkg_root, &full)?;
            }
        }
        Ok(())
    }
    walk(manager, unity_package_id, unity_pkg_path, unity_pkg_path)
}

use crate::config::{read_configs, resolve_origin_registry};
use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use flate2::{write::GzEncoder, Compression};
use reqwest::Client;
use serde_json::{json, Value};
use std::fs;
use std::io::Write;
use std::num::Wrapping;
use std::path::{Path, PathBuf};

fn read_package_manifest(dir: &Path) -> Result<(String, String, Value)> {
    let pkg_json_path = dir.join("package.json");
    if !pkg_json_path.exists() {
        bail!("no package.json found in {:?}", dir);
    }
    let pkg_raw = fs::read_to_string(&pkg_json_path)
        .with_context(|| format!("read {}", pkg_json_path.display()))?;
    let pkg: Value = serde_json::from_str(&pkg_raw).context("parse package.json")?;
    let name = pkg
        .get("name")
        .and_then(|v| v.as_str())
        .context("package.json missing \"name\"")?;
    let version = pkg
        .get("version")
        .and_then(|v| v.as_str())
        .context("package.json missing \"version\"")?;
    Ok((name.to_string(), version.to_string(), pkg))
}

/// Write a UPM/npm-style `.tgz` (files under `package/` in the archive). Default output: `Packages/<name>-<version>.tgz`.
pub fn pack_package_directory(dir: &Path, output: Option<&Path>) -> Result<PathBuf> {
    let (name, version, _) = read_package_manifest(dir)?;
    let bytes = build_tarball(dir)?;
    let file_name = format!("{name}-{version}.tgz");
    let out_path = if let Some(p) = output {
        p.to_path_buf()
    } else {
        PathBuf::from("Packages").join(&file_name)
    };
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&out_path, &bytes).with_context(|| format!("write {}", out_path.display()))?;
    Ok(out_path)
}

pub async fn publish_package(client: &Client, dir: &str, registry: Option<&str>) -> Result<()> {
    let pkg_dir = Path::new(dir);
    let (name, version, pkg) = read_package_manifest(pkg_dir)?;

    // 解析目标注册表
    let configs = read_configs()?;
    let (reg_name, reg_src) = if let Some(r) = registry {
        let src = configs
            .registry
            .origin
            .sources
            .get(r)
            .with_context(|| format!("unknown registry {:?}", r))?;
        (r.to_string(), src.clone())
    } else {
        let (n, s) = resolve_origin_registry(&name, &configs)?;
        (n.to_string(), s.clone())
    };
    let registry_url = reg_src.url.trim_end_matches('/');

    println!("Publishing {name}@{version} to {reg_name} ({registry_url})…");

    // 打 tarball（npm 约定：目录内容放在 package/ 前缀下）
    let tarball_bytes = build_tarball(pkg_dir)?;
    let tarball_b64 = B64.encode(&tarball_bytes);
    let shasum = sha1(&tarball_bytes);
    let tarball_name = format!("{name}-{version}.tgz");

    let version_latest = version.clone();
    let version_entry = version.clone();
    // npm publish PUT body
    let body = json!({
        "_id": name,
        "name": name,
        "description": pkg.get("description").and_then(|v| v.as_str()).unwrap_or(""),
        "dist-tags": { "latest": version_latest },
        "versions": {
            version_entry: {
                "name": name,
                "version": version,
                "description": pkg.get("description").and_then(|v| v.as_str()).unwrap_or(""),
                "main": pkg.get("main").and_then(|v| v.as_str()).unwrap_or(""),
                "unity": pkg.get("unity").and_then(|v| v.as_str()).unwrap_or("2021.3"),
                "author": pkg.get("author").cloned().unwrap_or(json!({})),
                "license": pkg.get("license").and_then(|v| v.as_str()).unwrap_or("MIT"),
                "dependencies": pkg.get("dependencies").cloned().unwrap_or(json!({})),
                "dist": {
                    "shasum": shasum,
                    "tarball": format!("{registry_url}/{name}/-/{tarball_name}"),
                }
            }
        },
        "_attachments": {
            tarball_name: {
                "content_type": "application/octet-stream",
                "data": tarball_b64,
                "length": tarball_bytes.len(),
            }
        }
    });

    let url = format!("{registry_url}/{}", urlencoded_name(&name));
    let mut req = client.put(&url).json(&body);
    if let Some(token) = &reg_src.token {
        req = req.bearer_auth(token);
    }
    let resp = req
        .send()
        .await
        .with_context(|| format!("PUT {url}"))?
        .error_for_status()
        .with_context(|| format!("registry rejected publish for {name}@{version}"))?;

    let status = resp.status();
    println!("Published {name}@{version}  (HTTP {})", status.as_u16());
    Ok(())
}

/// Build an in-memory .tgz with all files under `dir/` placed under `package/` prefix.
fn build_tarball(dir: &Path) -> Result<Vec<u8>> {
    let npmignore = load_npmignore_lines(dir);
    let buf = Vec::new();
    let enc = GzEncoder::new(buf, Compression::default());
    let mut tar = tar::Builder::new(enc);
    tar.follow_symlinks(false);
    append_dir_recursive(&mut tar, dir, dir, "package", &npmignore)?;
    let gz = tar.into_inner().context("finalize tar")?;
    gz.finish().context("finalize gzip")
}

fn load_npmignore_lines(dir: &Path) -> Vec<String> {
    let p = dir.join(".npmignore");
    if !p.is_file() {
        return Vec::new();
    }
    let Ok(raw) = fs::read_to_string(&p) else {
        return Vec::new();
    };
    raw.lines()
        .filter_map(|line| {
            let t = line.split('#').next().unwrap_or("").trim();
            if t.is_empty() {
                None
            } else {
                Some(t.replace('\\', "/"))
            }
        })
        .collect()
}

fn is_default_ignored_segment(seg: &str) -> bool {
    matches!(
        seg,
        ".git" | ".hg" | ".svn" | ".vs" | "node_modules" | "__pycache__" | ".idea"
    )
}

fn rule_excludes_rel(rel_posix: &str, rule: &str) -> bool {
    if rule.ends_with('/') {
        let p = rule.trim_end_matches('/');
        rel_posix == p || rel_posix.starts_with(&format!("{p}/"))
    } else {
        rel_posix == rule
            || rel_posix.starts_with(&format!("{rule}/"))
            || rel_posix.ends_with(&format!("/{rule}"))
    }
}

fn is_path_ignored_for_pack(rel_posix: &str, npmignore: &[String]) -> bool {
    let lower = rel_posix.to_ascii_lowercase();
    if lower == ".ds_store" || lower.ends_with("/.ds_store") {
        return true;
    }
    if lower == "thumbs.db" || lower.ends_with("/thumbs.db") {
        return true;
    }
    if rel_posix == ".npmignore" {
        return true;
    }
    for seg in rel_posix.split('/') {
        if is_default_ignored_segment(seg) {
            return true;
        }
    }
    npmignore
        .iter()
        .any(|rule| rule_excludes_rel(rel_posix, rule))
}

fn append_dir_recursive<W: Write>(
    tar: &mut tar::Builder<W>,
    base: &Path,
    current: &Path,
    prefix: &str,
    npmignore: &[String],
) -> Result<()> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let rel = path.strip_prefix(base).unwrap();
        let rel_posix = rel.to_string_lossy().replace('\\', "/");
        if is_path_ignored_for_pack(&rel_posix, npmignore) {
            continue;
        }
        let tar_path = format!("{prefix}/{}", rel_posix);
        if path.is_dir() {
            append_dir_recursive(tar, base, &path, prefix, npmignore)?;
        } else {
            let mut file =
                fs::File::open(&path).with_context(|| format!("open {}", path.display()))?;
            tar.append_file(&tar_path, &mut file)
                .with_context(|| format!("tar append {}", path.display()))?;
        }
    }
    Ok(())
}

fn sha1(data: &[u8]) -> String {
    // SHA-1 per FIPS 180-4
    let mut h: [Wrapping<u32>; 5] = [
        Wrapping(0x67452301),
        Wrapping(0xEFCDAB89),
        Wrapping(0x98BADCFE),
        Wrapping(0x10325476),
        Wrapping(0xC3D2E1F0),
    ];

    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0x00);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks(64) {
        let mut w = [Wrapping(0u32); 80];
        for i in 0..16 {
            w[i] = Wrapping(u32::from_be_bytes(
                chunk[i * 4..i * 4 + 4].try_into().unwrap(),
            ));
        }
        for i in 16..80 {
            let v = w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16];
            w[i] = Wrapping(v.0.rotate_left(1));
        }
        let [mut a, mut b, mut c, mut d, mut e] = h;
        for (i, wi) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | (!b & d), Wrapping(0x5A827999u32)),
                20..=39 => (b ^ c ^ d, Wrapping(0x6ED9EBA1u32)),
                40..=59 => ((b & c) | (b & d) | (c & d), Wrapping(0x8F1BBCDCu32)),
                _ => (b ^ c ^ d, Wrapping(0xCA62C1D6u32)),
            };
            let temp = Wrapping(a.0.rotate_left(5)) + f + e + k + *wi;
            e = d;
            d = c;
            c = Wrapping(b.0.rotate_left(30));
            b = a;
            a = temp;
        }
        h[0] += a;
        h[1] += b;
        h[2] += c;
        h[3] += d;
        h[4] += e;
    }

    format!(
        "{:08x}{:08x}{:08x}{:08x}{:08x}",
        h[0].0, h[1].0, h[2].0, h[3].0, h[4].0
    )
}

/// npm scoped packages use `@scope%2Fname` in the URL path.
fn urlencoded_name(name: &str) -> String {
    name.replace('/', "%2F")
}

#[cfg(test)]
mod tests {
    use super::{
        is_path_ignored_for_pack, load_npmignore_lines, pack_package_directory, rule_excludes_rel,
        sha1, urlencoded_name,
    };
    use anyhow::Result;
    use flate2::read::GzDecoder;
    use std::fs;
    use std::fs::File;
    use std::path::Path;

    fn tgz_file_entry_paths(path: &Path) -> Result<Vec<String>> {
        let f = File::open(path)?;
        let dec = GzDecoder::new(f);
        let mut ar = tar::Archive::new(dec);
        let mut v = Vec::new();
        for e in ar.entries()? {
            let e = e?;
            if e.header().entry_type().is_file() {
                v.push(e.path()?.to_string_lossy().replace('\\', "/"));
            }
        }
        Ok(v)
    }

    #[test]
    fn load_npmignore_parses_comments_and_blank_lines() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(
            tmp.path().join(".npmignore"),
            "# c\n\nfoo/\n  bar.txt  \n#tail\n",
        )
        .unwrap();
        let lines = load_npmignore_lines(tmp.path());
        assert_eq!(lines, vec!["foo/".to_string(), "bar.txt".to_string()]);
    }

    #[test]
    fn pack_tarball_respects_npmignore_and_excludes_vcs() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::write(
            root.join("package.json"),
            r#"{"name":"com.t.pack","version":"0.2.0"}"#,
        )
        .unwrap();
        fs::write(root.join("keep.txt"), "x").unwrap();
        fs::write(root.join("omit.txt"), "y").unwrap();
        fs::write(root.join(".npmignore"), "omit.txt\n").unwrap();
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(root.join(".git/config"), "[core]\n").unwrap();
        let out = root.join("out.tgz");
        pack_package_directory(root, Some(&out)).unwrap();
        let paths = tgz_file_entry_paths(&out).unwrap();
        assert!(paths.iter().any(|p| p.ends_with("package/package.json")));
        assert!(paths.iter().any(|p| p.ends_with("package/keep.txt")));
        assert!(!paths.iter().any(|p| p.contains("omit.txt")));
        assert!(!paths.iter().any(|p| p.contains(".git")));
    }

    #[test]
    fn pack_errors_without_package_json() {
        let tmp = tempfile::tempdir().unwrap();
        let out = tmp.path().join("out.tgz");
        let err = pack_package_directory(tmp.path(), Some(&out)).unwrap_err();
        assert!(
            err.to_string().contains("no package.json"),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    fn pack_errors_on_invalid_package_json() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("package.json"), "not json").unwrap();
        let err =
            pack_package_directory(tmp.path(), Some(&tmp.path().join("out.tgz"))).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("parse package.json"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn pack_errors_when_package_json_missing_name() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("package.json"), r#"{"version":"1.0.0"}"#).unwrap();
        let err =
            pack_package_directory(tmp.path(), Some(&tmp.path().join("out.tgz"))).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("package.json missing \"name\""),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn pack_errors_when_package_json_missing_version() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("package.json"), r#"{"name":"com.x"}"#).unwrap();
        let err =
            pack_package_directory(tmp.path(), Some(&tmp.path().join("out.tgz"))).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("package.json missing \"version\""),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn urlencoded_name_escapes_slash() {
        assert_eq!(urlencoded_name("@scope/pkg"), "@scope%2Fpkg");
        assert_eq!(urlencoded_name("com.plain"), "com.plain");
    }

    #[test]
    fn rule_excludes_directory_prefix() {
        assert!(rule_excludes_rel("Tests/foo", "Tests/"));
        assert!(!rule_excludes_rel("Other/foo", "Tests/"));
        assert!(rule_excludes_rel("Tests", "Tests/"));
    }

    #[test]
    fn rule_excludes_file_or_subpath() {
        assert!(rule_excludes_rel("secret", "secret"));
        assert!(rule_excludes_rel("dir/secret", "secret"));
        assert!(rule_excludes_rel("secret/file", "secret"));
    }

    #[test]
    fn pack_ignores_default_segments_and_ds_store() {
        let rules: Vec<String> = vec![];
        assert!(is_path_ignored_for_pack(".git/config", &rules));
        assert!(is_path_ignored_for_pack("pkg/.DS_Store", &rules));
        assert!(is_path_ignored_for_pack(".npmignore", &rules));
        assert!(!is_path_ignored_for_pack("package.json", &rules));
    }

    #[test]
    fn pack_respects_npmignore_rules() {
        let rules = vec!["tmp/".to_string(), "secret.txt".to_string()];
        assert!(is_path_ignored_for_pack("tmp/a", &rules));
        assert!(is_path_ignored_for_pack("x/secret.txt", &rules));
        assert!(!is_path_ignored_for_pack("src/lib.cs", &rules));
    }

    #[test]
    fn sha1_known_vectors() {
        assert_eq!(sha1(b""), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(sha1(b"abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
    }
}

use anyhow::{Context, Result};
use std::cmp::Ordering;

/// Prefer non-prerelease; pick highest by numeric dot segments (Unity / npm style).
pub fn pick_latest_stable(versions: &[String]) -> Result<String> {
    if versions.is_empty() {
        anyhow::bail!("no versions available");
    }
    let filtered: Vec<&String> = versions
        .iter()
        .filter(|v| {
            let l = v.to_lowercase();
            !l.contains("-preview")
                && !l.contains("-beta")
                && !l.contains("-rc")
                && !l.contains("-exp")
                && !l.contains("-pre")
                && !l.contains("-alpha")
        })
        .collect();
    let pool: Vec<&String> = if filtered.is_empty() {
        versions.iter().collect()
    } else {
        filtered
    };
    pool.iter()
        .max_by(|a, b| cmp_version_strings(a, b))
        .map(|s| (*s).clone())
        .context("no versions available")
}

pub fn cmp_version_strings(a: &str, b: &str) -> Ordering {
    let va = numeric_prefix_components(a);
    let vb = numeric_prefix_components(b);
    let n = va.len().max(vb.len());
    for i in 0..n {
        match va.get(i).unwrap_or(&0).cmp(vb.get(i).unwrap_or(&0)) {
            Ordering::Equal => {}
            o => return o,
        }
    }
    Ordering::Equal
}

fn numeric_prefix_components(s: &str) -> Vec<u32> {
    s.split(['.', '-', '_'])
        .filter_map(|part| part.parse::<u32>().ok())
        .collect()
}

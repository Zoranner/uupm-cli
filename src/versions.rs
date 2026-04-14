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

#[cfg(test)]
mod tests {
    use super::{cmp_version_strings, pick_latest_stable};
    use std::cmp::Ordering;

    #[test]
    fn pick_latest_prefers_non_prerelease() {
        let v = pick_latest_stable(&[
            "1.0.0-preview.1".to_string(),
            "0.9.0".to_string(),
            "1.0.0-beta.2".to_string(),
        ])
        .unwrap();
        assert_eq!(v, "0.9.0");
    }

    #[test]
    fn pick_latest_among_stables() {
        let v = pick_latest_stable(&[
            "1.2.3".to_string(),
            "2.0.0".to_string(),
            "1.9.9".to_string(),
        ])
        .unwrap();
        assert_eq!(v, "2.0.0");
    }

    #[test]
    fn pick_latest_only_prereleases() {
        let v = pick_latest_stable(&["1.0.0-preview.1".to_string(), "1.0.0-preview.2".to_string()])
            .unwrap();
        assert_eq!(v, "1.0.0-preview.2");
    }

    #[test]
    fn pick_latest_empty_errors() {
        assert!(pick_latest_stable(&[]).is_err());
    }

    #[test]
    fn cmp_version_numeric_segments() {
        assert_eq!(cmp_version_strings("2.0.0", "10.0.0"), Ordering::Less);
        assert_eq!(cmp_version_strings("10.0.0", "2.0.0"), Ordering::Greater);
        assert_eq!(cmp_version_strings("1.0.0", "1.0.0"), Ordering::Equal);
        assert_eq!(cmp_version_strings("1.0.1", "1.0.0"), Ordering::Greater);
    }
}

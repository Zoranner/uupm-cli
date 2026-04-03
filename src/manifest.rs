use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Manifest {
    pub dependencies: BTreeMap<String, String>,
    #[serde(default, rename = "scopedRegistries")]
    pub scoped_registries: Vec<ScopedRegistry>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScopedRegistry {
    pub name: String,
    pub url: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct RegistryPackageBody {
    #[serde(default)]
    pub versions: BTreeMap<String, VersionDetailJson>,
}

#[derive(Debug, Deserialize)]
pub struct VersionDetailJson {
    #[serde(default)]
    pub dependencies: BTreeMap<String, String>,
    pub dist: DistJson,
}

#[derive(Debug, Deserialize)]
pub struct DistJson {
    pub tarball: String,
}

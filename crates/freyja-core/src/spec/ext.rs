use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ExtensionsSpec {
    #[serde(default)]
    pub oci: Option<OciExtensionSpec>,
    #[serde(default)]
    pub alt_rpm: Option<AltRpmExtensionSpec>,
    #[serde(default)]
    pub apk: Option<ApkExtensionSpec>,
    #[serde(default)]
    pub deb: Option<DebExtensionSpec>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct OciExtensionSpec {
    #[serde(default)]
    pub enabled: bool,
    // Registry overrides are parsed for compatibility but not yet implemented.
    #[serde(default)]
    pub registry: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct AltRpmExtensionSpec {
    #[serde(default)]
    pub enabled: bool,
    // Repository overrides are parsed for compatibility but not yet implemented.
    #[serde(default)]
    pub repository: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ApkExtensionSpec {
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct DebExtensionSpec {
    #[serde(default)]
    pub enabled: bool,
}

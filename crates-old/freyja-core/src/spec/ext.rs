use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ExtensionsSpec {
    #[serde(default)]
    pub oci: Option<OciExtensionSpec>,

    #[serde(default)]
    pub alt_rpm: Option<AltRpmExtensionSpec>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OciExtensionSpec {
    pub enabled: bool,

    #[serde(default)]
    pub registry: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AltRpmExtensionSpec {
    pub enabled: bool,

    #[serde(default)]
    pub repository: Option<String>,
}

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BuildSpec {
    pub context: PathBuf,

    #[serde(default = "default_dockerfile")]
    pub dockerfile: PathBuf,
}

fn default_dockerfile() -> PathBuf {
    PathBuf::from("Dockerfile")
}

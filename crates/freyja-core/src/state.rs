use crate::error::Error;
use crate::spec::deps::ResolvedDependency;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct State {
    pub targets: BTreeMap<String, TargetState>,
}

impl State {
    pub fn load(path: &Path) -> Result<Self, Error> {
        let contents = fs::read_to_string(path)?;
        Ok(
            toml::from_str(&contents).map_err(|source| Error::ParseState {
                path: path.to_path_buf(),
                source,
            })?,
        )
    }

    pub fn save(&self, path: &Path) -> Result<(), Error> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content =
            toml::to_string_pretty(self).map_err(|source| Error::SerializeState { source })?;
        let temp_path = temporary_path(path);
        fs::write(&temp_path, content)?;
        fs::rename(&temp_path, path)?;
        Ok(())
    }

    pub fn record_build(
        &mut self,
        name: String,
        dependencies: BTreeMap<String, ResolvedDependency>,
        build_fingerprint: String,
    ) {
        self.targets.insert(
            name,
            TargetState {
                dependencies,
                build_fingerprint: Some(build_fingerprint),
            },
        );
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TargetState {
    pub dependencies: BTreeMap<String, ResolvedDependency>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build_fingerprint: Option<String>,
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");

    PathBuf::from(tmp)
}

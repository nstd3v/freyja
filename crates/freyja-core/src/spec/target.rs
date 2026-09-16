use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::spec::build::BuildSpec;

use super::deps::DependencySpec;

#[derive(Debug, Deserialize, Serialize)]
pub struct TargetSpec {
    pub image: String,
    pub tags: Vec<String>,
    pub arches: Vec<Arches>,
    pub build: BuildSpec,
    pub dependencies: BTreeMap<String, DependencySpec>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Arches {
    Amd64,
    Arm64,
}

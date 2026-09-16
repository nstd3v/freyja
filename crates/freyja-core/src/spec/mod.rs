pub mod deps;
pub mod ext;
pub mod target;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize, Serialize)]
pub struct Spec {
    pub version: String,
    pub title: String,
    pub extensions: ext::ExtensionsSpec,
    pub targets: BTreeMap<String, target::TargetSpec>,
}

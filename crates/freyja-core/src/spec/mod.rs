pub mod build;
pub mod deps;
pub mod ext;
pub mod target;

pub use build::BuildSpec;
pub use deps::*;
pub use ext::*;
pub use target::*;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize, Serialize)]
pub struct Spec {
    pub version: String,
    pub title: String,
    #[serde(default)]
    pub extensions: ext::ExtensionsSpec,
    pub targets: BTreeMap<String, target::TargetSpec>,
}

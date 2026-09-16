use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::spec::deps::ResolvedDependency;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct State {
    pub targets: BTreeMap<String, TargetState>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TargetState {
    pub dependencies: BTreeMap<String, ResolvedDependency>,
}

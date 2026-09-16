use std::collections::BTreeMap;

use crate::spec::deps::ResolvedDependency;

#[derive(Debug, Clone)]
pub struct Plan {
    pub targets: Vec<TargetPlan>,
}

#[derive(Debug, Clone)]
pub struct TargetPlan {
    pub target: String,
    pub action: PlanAction,
    pub reasons: Vec<PlanReason>,
    pub dependencies: BTreeMap<String, ResolvedDependency>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanAction {
    Build,
    Skip,
}

#[derive(Debug, Clone)]
pub enum PlanReason {
    NeverBuilt,

    DependencyChanged {
        name: String,
        kind: String,
        reference: String,
        previous_fingerprint: String,
        current_fingerprint: String,
    },

    DependencyAdded {
        name: String,
        kind: String,
        reference: String,
    },

    DependencyRemoved {
        name: String,
        kind: String,
        reference: String,
    },
}

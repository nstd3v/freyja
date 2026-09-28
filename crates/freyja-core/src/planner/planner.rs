use std::collections::BTreeMap;

use super::{Plan, PlanAction, PlanReason, TargetPlan};
use crate::{
    error::Error,
    fingerprint::fingerprint,
    resolver::ResolverRegistry,
    spec::{Spec, deps::ResolvedDependency},
    state::State,
};

pub struct Planner<'a> {
    resolvers: &'a ResolverRegistry,
}

impl<'a> Planner<'a> {
    pub fn new(resolvers: &'a ResolverRegistry) -> Self {
        Self { resolvers }
    }

    pub async fn plan(&self, spec: &Spec, state: &State) -> Result<Plan, Error> {
        let mut targets = Vec::new();

        for (name, target) in &spec.targets {
            let build_fingerprint = fingerprint(target)?;
            let mut resolved = BTreeMap::new();

            for (name, dependency) in &target.dependencies {
                let resolved_dependency = self.resolvers.resolve(dependency).await?;

                resolved.insert(name.clone(), resolved_dependency);
            }

            let previous = state.targets.get(name);

            let reasons = match previous {
                None => {
                    vec![PlanReason::NeverBuilt]
                }

                Some(previous) => {
                    let mut reasons = compare_dependencies(&previous.dependencies, &resolved);
                    if previous.build_fingerprint.as_deref() != Some(&build_fingerprint) {
                        reasons.push(PlanReason::BuildInputChanged);
                    }
                    reasons
                }
            };

            let action = if reasons.is_empty() {
                PlanAction::Skip
            } else {
                PlanAction::Build
            };

            targets.push(TargetPlan {
                target: name.clone(),
                action,
                reasons,
                dependencies: resolved,
                build_fingerprint,
            });
        }

        Ok(Plan { targets })
    }
}

fn compare_dependencies(
    previous: &BTreeMap<String, ResolvedDependency>,
    current: &BTreeMap<String, ResolvedDependency>,
) -> Vec<PlanReason> {
    let mut reasons = Vec::new();

    for (name, current_dep) in current {
        match previous.get(name) {
            None => {
                reasons.push(PlanReason::DependencyAdded {
                    name: name.clone(),
                    kind: current_dep.kind.clone(),
                    reference: current_dep.reference.clone(),
                });
            }

            Some(previous_dep) if previous_dep.fingerprint != current_dep.fingerprint => {
                reasons.push(PlanReason::DependencyChanged {
                    name: name.clone(),
                    kind: current_dep.kind.clone(),
                    reference: current_dep.reference.clone(),
                    previous_fingerprint: previous_dep.fingerprint.clone(),
                    current_fingerprint: current_dep.fingerprint.clone(),
                });
            }

            Some(_) => {}
        }
    }

    for (name, previous_dep) in previous {
        if !current.contains_key(name) {
            reasons.push(PlanReason::DependencyRemoved {
                name: name.clone(),
                kind: previous_dep.kind.clone(),
                reference: previous_dep.reference.clone(),
            });
        }
    }

    reasons
}

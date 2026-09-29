mod models;
mod planner;

pub use models::*;
pub use planner::Planner;

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashMap};

    use async_trait::async_trait;
    use toml::Table;

    use crate::{
        error::Error,
        planner::{PlanAction, PlanReason, Planner},
        resolver::ResolverRegistry,
        spec::{
            Spec,
            build::BuildSpec,
            deps::{DependencyResolver, DependencySpec, ResolvedDependency},
            ext::ExtensionsSpec,
            target::TargetSpec,
        },
        state::{State, TargetState},
    };

    struct FakeResolver {
        fingerprints: HashMap<String, String>,
    }

    impl FakeResolver {
        fn new<const N: usize>(items: [(&str, &str); N]) -> Self {
            Self {
                fingerprints: items
                    .into_iter()
                    .map(|(reference, fingerprint)| (reference.to_owned(), fingerprint.to_owned()))
                    .collect(),
            }
        }
    }

    #[async_trait]
    impl DependencyResolver for FakeResolver {
        fn kind(&self) -> &'static str {
            "fake"
        }

        async fn resolve(&self, dependency: &DependencySpec) -> Result<ResolvedDependency, Error> {
            let reference = dependency
                .config
                .get("ref")
                .and_then(toml::Value::as_str)
                .expect("fake dependency must contain `ref`");

            let fingerprint = self
                .fingerprints
                .get(reference)
                .expect("fake resolver must contain fingerprint");

            Ok(ResolvedDependency {
                kind: "fake".to_owned(),
                reference: reference.to_owned(),
                fingerprint: fingerprint.clone(),
                metadata: Table::new(),
            })
        }
    }

    fn dependency(reference: &str) -> DependencySpec {
        let mut config = Table::new();

        config.insert("ref".to_owned(), toml::Value::String(reference.to_owned()));

        DependencySpec {
            kind: "fake".to_owned(),
            config,
        }
    }

    fn dependency_with_kind(kind: &str, reference: &str) -> DependencySpec {
        let mut config = Table::new();

        config.insert("ref".to_owned(), toml::Value::String(reference.to_owned()));

        DependencySpec {
            kind: kind.to_owned(),
            config,
        }
    }

    fn dependencies<const N: usize>(
        items: [(&str, DependencySpec); N],
    ) -> BTreeMap<String, DependencySpec> {
        items
            .into_iter()
            .map(|(name, dependency)| (name.to_owned(), dependency))
            .collect()
    }

    fn resolved(reference: &str, fingerprint: &str) -> ResolvedDependency {
        ResolvedDependency {
            kind: "fake".to_owned(),
            reference: reference.to_owned(),
            fingerprint: fingerprint.to_owned(),
            metadata: Table::new(),
        }
    }

    fn resolved_dependencies<const N: usize>(
        items: [(&str, ResolvedDependency); N],
    ) -> BTreeMap<String, ResolvedDependency> {
        items
            .into_iter()
            .map(|(name, dependency)| (name.to_owned(), dependency))
            .collect()
    }

    fn spec_with_dependencies(dependencies: BTreeMap<String, DependencySpec>) -> Spec {
        let mut targets = BTreeMap::new();

        targets.insert(
            "app".to_owned(),
            TargetSpec {
                image: "example.com/app".to_owned(),
                tags: Vec::new(),
                arches: Vec::new(),

                build: BuildSpec {
                    context: ".".into(),
                    dockerfile: "Dockerfile".into(),
                },

                dependencies,
            },
        );

        Spec {
            version: "1".to_owned(),
            title: "test".to_owned(),
            extensions: ExtensionsSpec::default(),
            targets,
        }
    }

    fn state_with_dependencies(dependencies: BTreeMap<String, ResolvedDependency>) -> State {
        let mut targets = BTreeMap::new();

        targets.insert("app".to_owned(), TargetState { dependencies });

        State { targets }
    }

    fn registry_with(resolver: FakeResolver) -> ResolverRegistry {
        let mut registry = ResolverRegistry::new();
        registry.register(resolver);
        registry
    }

    #[tokio::test]
    async fn never_built_target_is_scheduled_for_build() {
        let spec = spec_with_dependencies(dependencies([("base", dependency("foo:latest"))]));

        let state = State::default();

        let registry = registry_with(FakeResolver::new([("foo:latest", "sha256:aaa")]));

        let planner = Planner::new(&registry);

        let plan = planner.plan(&spec, &state).await.unwrap();

        assert_eq!(plan.targets.len(), 1);

        let target = &plan.targets[0];

        assert_eq!(target.target, "app");
        assert_eq!(target.action, PlanAction::Build);

        assert!(matches!(
            target.reasons.as_slice(),
            [PlanReason::NeverBuilt]
        ));
    }

    #[tokio::test]
    async fn unchanged_dependencies_are_skipped() {
        let spec = spec_with_dependencies(dependencies([("base", dependency("foo:latest"))]));

        let state = state_with_dependencies(resolved_dependencies([(
            "base",
            resolved("foo:latest", "sha256:aaa"),
        )]));

        let registry = registry_with(FakeResolver::new([("foo:latest", "sha256:aaa")]));

        let planner = Planner::new(&registry);

        let plan = planner.plan(&spec, &state).await.unwrap();

        let target = &plan.targets[0];

        assert_eq!(target.action, PlanAction::Skip);
        assert!(target.reasons.is_empty());
    }

    #[tokio::test]
    async fn changed_fingerprint_schedules_build() {
        let spec = spec_with_dependencies(dependencies([("base", dependency("foo:latest"))]));

        let state = state_with_dependencies(resolved_dependencies([(
            "base",
            resolved("foo:latest", "sha256:aaa"),
        )]));

        let registry = registry_with(FakeResolver::new([("foo:latest", "sha256:bbb")]));

        let planner = Planner::new(&registry);

        let plan = planner.plan(&spec, &state).await.unwrap();

        let target = &plan.targets[0];

        assert_eq!(target.action, PlanAction::Build);

        assert!(matches!(
            target.reasons.as_slice(),
            [
                PlanReason::DependencyChanged {
                    name,
                    kind,
                    reference,
                    previous_fingerprint,
                    current_fingerprint,
                }
            ] if
                name == "base"
                && kind == "fake"
                && reference == "foo:latest"
                && previous_fingerprint == "sha256:aaa"
                && current_fingerprint == "sha256:bbb"
        ));
    }

    #[tokio::test]
    async fn added_dependency_schedules_build() {
        let spec = spec_with_dependencies(dependencies([
            ("base", dependency("foo:latest")),
            ("runtime", dependency("bar:latest")),
        ]));

        let state = state_with_dependencies(resolved_dependencies([(
            "base",
            resolved("foo:latest", "sha256:aaa"),
        )]));

        let registry = registry_with(FakeResolver::new([
            ("foo:latest", "sha256:aaa"),
            ("bar:latest", "sha256:bbb"),
        ]));

        let planner = Planner::new(&registry);

        let plan = planner.plan(&spec, &state).await.unwrap();

        let target = &plan.targets[0];

        assert_eq!(target.action, PlanAction::Build);

        assert_eq!(target.reasons.len(), 1);

        assert!(matches!(
            target.reasons.as_slice(),
            [
                PlanReason::DependencyAdded {
                    name,
                    kind,
                    reference,
                }
            ] if
                name == "runtime"
                && kind == "fake"
                && reference == "bar:latest"
        ));
    }

    #[tokio::test]
    async fn removed_dependency_schedules_build() {
        let spec = spec_with_dependencies(dependencies([("base", dependency("foo:latest"))]));

        let state = state_with_dependencies(resolved_dependencies([
            ("base", resolved("foo:latest", "sha256:aaa")),
            ("runtime", resolved("bar:latest", "sha256:bbb")),
        ]));

        let registry = registry_with(FakeResolver::new([("foo:latest", "sha256:aaa")]));

        let planner = Planner::new(&registry);

        let plan = planner.plan(&spec, &state).await.unwrap();

        let target = &plan.targets[0];

        assert_eq!(target.action, PlanAction::Build);

        assert_eq!(target.reasons.len(), 1);

        assert!(matches!(
            target.reasons.as_slice(),
            [
                PlanReason::DependencyRemoved {
                    name,
                    kind,
                    reference,
                }
            ] if
                name == "runtime"
                && kind == "fake"
                && reference == "bar:latest"
        ));
    }

    #[tokio::test]
    async fn dependency_order_does_not_trigger_rebuild() {
        /*
         * BTreeMap sorts keys, so insertion order is deliberately
         * irrelevant to planner semantics.
         */

        let spec_dependencies = [
            ("runtime", dependency("bar:latest")),
            ("base", dependency("foo:latest")),
        ]
        .into_iter()
        .map(|(name, dependency)| (name.to_owned(), dependency))
        .collect();

        let state_dependencies = [
            ("base", resolved("foo:latest", "sha256:aaa")),
            ("runtime", resolved("bar:latest", "sha256:bbb")),
        ]
        .into_iter()
        .map(|(name, dependency)| (name.to_owned(), dependency))
        .collect();

        let spec = spec_with_dependencies(spec_dependencies);

        let state = state_with_dependencies(state_dependencies);

        let registry = registry_with(FakeResolver::new([
            ("foo:latest", "sha256:aaa"),
            ("bar:latest", "sha256:bbb"),
        ]));

        let planner = Planner::new(&registry);

        let plan = planner.plan(&spec, &state).await.unwrap();

        let target = &plan.targets[0];

        assert_eq!(target.action, PlanAction::Skip);
        assert!(target.reasons.is_empty());
    }

    #[tokio::test]
    async fn multiple_dependency_changes_are_all_reported() {
        let spec = spec_with_dependencies(dependencies([
            ("base", dependency("foo:latest")),
            ("runtime", dependency("bar:latest")),
        ]));

        let state = state_with_dependencies(resolved_dependencies([
            ("base", resolved("foo:latest", "sha256:old-foo")),
            ("runtime", resolved("bar:latest", "sha256:old-bar")),
        ]));

        let registry = registry_with(FakeResolver::new([
            ("foo:latest", "sha256:new-foo"),
            ("bar:latest", "sha256:new-bar"),
        ]));

        let planner = Planner::new(&registry);

        let plan = planner.plan(&spec, &state).await.unwrap();

        let target = &plan.targets[0];

        assert_eq!(target.action, PlanAction::Build);
        assert_eq!(target.reasons.len(), 2);

        assert!(
            target
                .reasons
                .iter()
                .all(|reason| matches!(reason, PlanReason::DependencyChanged { .. }))
        );

        assert!(target.reasons.iter().any(|reason| matches!(
            reason,
            PlanReason::DependencyChanged {
                name,
                previous_fingerprint,
                current_fingerprint,
                ..
            } if
                name == "base"
                && previous_fingerprint
                    == "sha256:old-foo"
                && current_fingerprint
                    == "sha256:new-foo"
        )));

        assert!(target.reasons.iter().any(|reason| matches!(
            reason,
            PlanReason::DependencyChanged {
                name,
                previous_fingerprint,
                current_fingerprint,
                ..
            } if
                name == "runtime"
                && previous_fingerprint
                    == "sha256:old-bar"
                && current_fingerprint
                    == "sha256:new-bar"
        )));
    }

    #[tokio::test]
    async fn unknown_dependency_type_returns_error() {
        let spec = spec_with_dependencies(dependencies([(
            "base",
            dependency_with_kind("unknown", "foo:latest"),
        )]));

        let state = State::default();

        let registry = ResolverRegistry::new();

        let planner = Planner::new(&registry);

        let result = planner.plan(&spec, &state).await;

        assert!(matches!(
            result,
            Err(Error::UnknownDependencyType { kind })
                if kind == "unknown"
        ));
    }
}

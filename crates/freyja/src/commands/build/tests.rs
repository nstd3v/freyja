use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

use freyja_core::{
    builder::Builder,
    error::Error,
    planner::{PlanAction, Planner},
    resolver::ResolverRegistry,
    spec::{DependencySpec, ResolvedDependency, TargetSpec, deps::DependencyResolver},
    state::State,
};

use super::execute;
use crate::helpers::load_spec;

struct TestDir(PathBuf);

impl TestDir {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "freyja-build-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).expect("create isolated test directory");
        Self(path)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).expect("remove test directory");
    }
}

struct FakeResolver(Arc<Mutex<String>>);

#[async_trait::async_trait]
impl DependencyResolver for FakeResolver {
    fn kind(&self) -> &'static str {
        "fake"
    }

    async fn resolve(&self, _: &DependencySpec) -> Result<ResolvedDependency, Error> {
        Ok(ResolvedDependency {
            kind: "fake".into(),
            reference: "test-input".into(),
            fingerprint: self.0.lock().unwrap().clone(),
            metadata: Default::default(),
        })
    }
}

struct FakeBuilder {
    calls: AtomicUsize,
    fail: AtomicBool,
}

#[async_trait::async_trait]
impl Builder for FakeBuilder {
    async fn build(&self, _: &str, _: &TargetSpec) -> Result<(), Error> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        if self.fail.load(Ordering::Relaxed) {
            return Err(Error::BuildFailed {
                target: "app".into(),
                status: Some(1),
            });
        }
        Ok(())
    }
}

fn fixture(
    dir: &TestDir,
) -> (
    PathBuf,
    PathBuf,
    ResolverRegistry,
    Arc<Mutex<String>>,
    FakeBuilder,
) {
    let spec_path = dir.path("freyja.toml");
    let context = dir.path("context");
    std::fs::create_dir(&context).unwrap();
    std::fs::write(context.join("Dockerfile"), "FROM scratch\n").unwrap();
    std::fs::write(
        &spec_path,
        format!(r#"version = "1"
title = "test"
[extensions]
[targets.app]
image = "example/app"
tags = ["latest"]
arches = ["amd64"]
build.context = "{}"
[targets.app.dependencies.input]
type = "fake"
"#, context.display()),
    )
    .unwrap();
    let fingerprint = Arc::new(Mutex::new("v1".to_owned()));
    let mut registry = ResolverRegistry::new();
    registry.register(FakeResolver(Arc::clone(&fingerprint)));
    let builder = FakeBuilder {
        calls: AtomicUsize::new(0),
        fail: AtomicBool::new(false),
    };
    (
        spec_path,
        dir.path("state.toml"),
        registry,
        fingerprint,
        builder,
    )
}

fn saved_fingerprint(path: &Path) -> String {
    State::load(path).unwrap().targets["app"].dependencies["input"]
        .fingerprint
        .clone()
}

#[tokio::test]
async fn first_build_saves_resolved_input_then_unchanged_input_skips() {
    let dir = TestDir::new();
    let (spec_path, state_path, registry, _, builder) = fixture(&dir);
    let planner = Planner::new(&registry);

    let initial = planner
        .plan(&load_spec(&spec_path).unwrap(), &State::default())
        .await
        .unwrap();
    assert_eq!(initial.targets[0].action, PlanAction::Build);

    execute(&spec_path, &state_path, &planner, &builder)
        .await
        .unwrap();
    assert_eq!(builder.calls.load(Ordering::Relaxed), 1);
    assert_eq!(saved_fingerprint(&state_path), "v1");

    let next = planner
        .plan(
            &load_spec(&spec_path).unwrap(),
            &State::load(&state_path).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(next.targets[0].action, PlanAction::Skip);
    execute(&spec_path, &state_path, &planner, &builder)
        .await
        .unwrap();
    assert_eq!(builder.calls.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn failed_rebuild_does_not_advance_saved_state() {
    let dir = TestDir::new();
    let (spec_path, state_path, registry, fingerprint, builder) = fixture(&dir);
    let planner = Planner::new(&registry);

    execute(&spec_path, &state_path, &planner, &builder)
        .await
        .unwrap();
    let before = std::fs::read(&state_path).unwrap();
    *fingerprint.lock().unwrap() = "v2".into();
    builder.fail.store(true, Ordering::Relaxed);

    assert!(matches!(
        execute(&spec_path, &state_path, &planner, &builder).await,
        Err(Error::BuildFailed { .. })
    ));
    assert_eq!(builder.calls.load(Ordering::Relaxed), 2);
    assert_eq!(std::fs::read(&state_path).unwrap(), before);
    assert_eq!(saved_fingerprint(&state_path), "v1");

    builder.fail.store(false, Ordering::Relaxed);
    execute(&spec_path, &state_path, &planner, &builder)
        .await
        .unwrap();
    assert_eq!(builder.calls.load(Ordering::Relaxed), 3);
    assert_eq!(saved_fingerprint(&state_path), "v2");
}

#[tokio::test]
async fn changed_dockerfile_rebuilds_with_unchanged_dependency() {
    let dir = TestDir::new();
    let (spec_path, state_path, registry, _, builder) = fixture(&dir);
    let planner = Planner::new(&registry);

    execute(&spec_path, &state_path, &planner, &builder).await.unwrap();
    std::fs::write(dir.path("context/Dockerfile"), "FROM busybox\n").unwrap();
    execute(&spec_path, &state_path, &planner, &builder).await.unwrap();

    assert_eq!(builder.calls.load(Ordering::Relaxed), 2);
}

#[tokio::test]
async fn context_file_changes_rebuild_and_then_skip() {
    let dir = TestDir::new();
    let (spec_path, state_path, registry, _, builder) = fixture(&dir);
    let planner = Planner::new(&registry);
    let source = dir.path("context/source.txt");

    execute(&spec_path, &state_path, &planner, &builder).await.unwrap();
    std::fs::write(&source, "first").unwrap();
    execute(&spec_path, &state_path, &planner, &builder).await.unwrap();
    std::fs::write(&source, "second").unwrap();
    execute(&spec_path, &state_path, &planner, &builder).await.unwrap();
    std::fs::remove_file(&source).unwrap();
    execute(&spec_path, &state_path, &planner, &builder).await.unwrap();
    execute(&spec_path, &state_path, &planner, &builder).await.unwrap();
    assert_eq!(builder.calls.load(Ordering::Relaxed), 4);
}

#[tokio::test]
async fn changed_target_configuration_and_legacy_state_rebuild() {
    let dir = TestDir::new();
    let (spec_path, state_path, registry, _, builder) = fixture(&dir);
    let planner = Planner::new(&registry);

    execute(&spec_path, &state_path, &planner, &builder).await.unwrap();
    let old = std::fs::read_to_string(&spec_path).unwrap();
    std::fs::write(&spec_path, old.replace("example/app", "example/other")).unwrap();
    execute(&spec_path, &state_path, &planner, &builder).await.unwrap();
    assert_eq!(builder.calls.load(Ordering::Relaxed), 2);

    let mut state = State::load(&state_path).unwrap();
    state.targets.get_mut("app").unwrap().build_fingerprint = None;
    state.save(&state_path).unwrap();
    execute(&spec_path, &state_path, &planner, &builder).await.unwrap();
    execute(&spec_path, &state_path, &planner, &builder).await.unwrap();
    assert_eq!(builder.calls.load(Ordering::Relaxed), 3);
}

#[tokio::test]
async fn state_inside_context_is_rejected_before_building() {
    let dir = TestDir::new();
    let (spec_path, _, registry, _, builder) = fixture(&dir);
    let planner = Planner::new(&registry);
    let state_path = dir.path("context/.freyja/state.toml");

    assert!(matches!(
        execute(&spec_path, &state_path, &planner, &builder).await,
        Err(Error::InvalidBuildInput { .. })
    ));
    assert_eq!(builder.calls.load(Ordering::Relaxed), 0);
}

#[tokio::test]
async fn dockerfile_outside_context_is_rejected() {
    let dir = TestDir::new();
    let (spec_path, state_path, registry, _, builder) = fixture(&dir);
    let planner = Planner::new(&registry);
    std::fs::write(dir.path("outside"), "FROM scratch\n").unwrap();
    let old = std::fs::read_to_string(&spec_path).unwrap();
    std::fs::write(
        &spec_path,
        old.replace("build.context =", "build.dockerfile = \"../outside\"\nbuild.context ="),
    ).unwrap();

    assert!(matches!(
        execute(&spec_path, &state_path, &planner, &builder).await,
        Err(Error::InvalidBuildInput { .. })
    ));
    assert_eq!(builder.calls.load(Ordering::Relaxed), 0);
}

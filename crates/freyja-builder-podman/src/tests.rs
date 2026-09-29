use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

use freyja_core::{
    builder::Builder,
    spec::{BuildSpec, TargetSpec},
};

use super::BuildkitBuilder;

struct TestDir(PathBuf);
impl TestDir {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "freyja-podman-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for TestDir {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[tokio::test]
async fn nondefault_dockerfile_is_passed_as_absolute_path() {
    let dir = TestDir::new();
    let output = dir.0.join("args");
    let podman = dir.0.join("fake-podman");
    fs::write(
        &podman,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\n",
            output.display()
        ),
    )
    .unwrap();
    let mut permissions = fs::metadata(&podman).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&podman, permissions).unwrap();
    let context = dir.0.join("context");
    fs::create_dir(&context).unwrap();
    fs::write(context.join("Containerfile.alt"), "FROM scratch\n").unwrap();
    let target = TargetSpec {
        image: "example/app".into(),
        tags: vec!["latest".into()],
        arches: vec![],
        build: BuildSpec {
            context: context.clone(),
            dockerfile: "Containerfile.alt".into(),
        },
        dependencies: BTreeMap::new(),
    };

    BuildkitBuilder::with_program(podman)
        .build("app", &target)
        .await
        .unwrap();
    let args = fs::read_to_string(output).unwrap();
    let lines: Vec<_> = args.lines().collect();
    assert_eq!(
        lines,
        [
            "buildx",
            "build",
            "-f",
            context.join("Containerfile.alt").to_str().unwrap(),
            context.to_str().unwrap(),
            "-t",
            "example/app",
            "--load"
        ]
    );
}

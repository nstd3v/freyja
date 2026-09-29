use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

use super::load_spec;
use freyja_core::error::Error;

struct TestDir(PathBuf);
impl TestDir {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "freyja-path-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn spec(&self, context: &str, dockerfile: &str) -> PathBuf {
        let path = self.0.join("freyja.toml");
        fs::write(
            &path,
            format!(
                r#"version = "1"
title = "paths"
[extensions]
[targets.app]
image = "example/app"
tags = ["latest"]
arches = ["amd64"]
build.context = "{context}"
build.dockerfile = "{dockerfile}"
[targets.app.dependencies]
"#
            ),
        )
        .unwrap();
        path
    }
}
impl Drop for TestDir {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn nondefault_dockerfile_is_relative_to_context() {
    let dir = TestDir::new();
    fs::create_dir_all(dir.0.join("context/sub")).unwrap();
    fs::write(dir.0.join("context/sub/Containerfile"), "FROM scratch\n").unwrap();
    let build = &load_spec(&dir.spec("context", "sub/Containerfile"))
        .unwrap()
        .targets["app"]
        .build;
    assert_eq!(build.context, dir.0.join("context").canonicalize().unwrap());
    assert_eq!(build.dockerfile, Path::new("sub/Containerfile"));
}

#[test]
fn missing_context_and_dockerfile_report_errors() {
    let dir = TestDir::new();
    assert!(matches!(
        load_spec(&dir.spec("missing", "Dockerfile")),
        Err(Error::BuildInput { .. })
    ));
    fs::create_dir(dir.0.join("context")).unwrap();
    assert!(matches!(
        load_spec(&dir.spec("context", "Dockerfile")),
        Err(Error::BuildInput { .. })
    ));
}

#[test]
fn absolute_and_escaping_dockerfile_paths_are_rejected() {
    let dir = TestDir::new();
    fs::create_dir(dir.0.join("context")).unwrap();
    fs::write(dir.0.join("outside"), "FROM scratch\n").unwrap();
    for dockerfile in ["../outside", dir.0.join("outside").to_str().unwrap()] {
        assert!(matches!(
            load_spec(&dir.spec("context", dockerfile)),
            Err(Error::InvalidBuildInput { .. })
        ));
    }
}

#[cfg(unix)]
#[test]
fn symlinked_dockerfile_cannot_escape_context() {
    let dir = TestDir::new();
    fs::create_dir(dir.0.join("context")).unwrap();
    fs::write(dir.0.join("outside"), "FROM scratch\n").unwrap();
    std::os::unix::fs::symlink(dir.0.join("outside"), dir.0.join("context/Dockerfile")).unwrap();
    assert!(matches!(
        load_spec(&dir.spec("context", "Dockerfile")),
        Err(Error::InvalidBuildInput { .. })
    ));
}

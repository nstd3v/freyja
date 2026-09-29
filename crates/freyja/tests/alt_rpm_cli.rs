use std::{
    fs,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

#[test]
fn cli_routes_alt_rpm_dependencies_to_resolver() {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "freyja-alt-rpm-cli-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&dir).unwrap();
    fs::write(dir.join("Dockerfile"), "FROM scratch\n").unwrap();
    fs::write(
        dir.join("freyja.toml"),
        r#"version = "1"
title = "ALT RPM wiring"
[extensions]
alt_rpm.enabled = true
[targets.app]
image = "example/app"
tags = ["latest"]
arches = ["amd64"]
build.context = "."
[targets.app.dependencies.package]
type = "alt_rpm"
repository = "nonexistent"
arch = "x86_64"
package = "nginx"
"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_freyja"))
        .arg("--file")
        .arg(dir.join("freyja.toml"))
        .arg("--state")
        .arg(
            dir.parent()
                .unwrap()
                .join("freyja-alt-rpm-unused-state.toml"),
        )
        .arg("--dir")
        .arg(dir.join("cache"))
        .arg("plan")
        .output()
        .unwrap();
    fs::remove_dir_all(&dir).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(!output.status.success());
    assert!(
        stderr.contains("unknown ALT RPM repository `nonexistent`"),
        "{stderr}"
    );
}

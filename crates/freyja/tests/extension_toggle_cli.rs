use std::{
    fs,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

fn plan(extensions: &str, kind: &str, config: &str) -> String {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "freyja-extension-toggle-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&dir).unwrap();
    fs::write(dir.join("Dockerfile"), "FROM scratch\n").unwrap();
    fs::write(
        dir.join("freyja.toml"),
        format!(
            r#"version = "1"
title = "extension toggle"
{extensions}
[targets.app]
image = "example/app"
tags = ["latest"]
arches = ["amd64"]
build.context = "."
[targets.app.dependencies.input]
type = "{kind}"
{config}
"#
        ),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_freyja"))
        .arg("--file")
        .arg(dir.join("freyja.toml"))
        .arg("--state")
        .arg(dir.with_extension("state.toml"))
        .arg("--dir")
        .arg(dir.join(".freyja"))
        .arg("plan")
        .output()
        .unwrap();
    fs::remove_dir_all(dir).unwrap();
    assert!(
        !output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    String::from_utf8(output.stderr).unwrap()
}

#[test]
fn omitted_extensions_are_disabled() {
    for (kind, config) in [
        (
            "apk",
            "release = \"v3.24\"\nrepository = \"testing\"\narch = \"x86_64\"\npackage = \"nginx\"",
        ),
        (
            "alt_rpm",
            "repository = \"missing\"\narch = \"x86_64\"\npackage = \"nginx\"",
        ),
        ("oci", "ref = \"invalid ref\""),
    ] {
        for extensions in ["[extensions]", ""] {
            let stderr = plan(extensions, kind, config);
            assert!(
                stderr.contains(&format!("unknown dependency type: {kind}")),
                "{stderr}"
            );
        }
    }
}

#[test]
fn explicitly_disabled_apk_is_not_registered() {
    let stderr = plan(
        "[extensions]\napk.enabled = false",
        "apk",
        "release = \"v3.24\"\nrepository = \"testing\"\narch = \"x86_64\"\npackage = \"nginx\"",
    );
    assert!(stderr.contains("unknown dependency type: apk"), "{stderr}");
}

#[test]
fn enabled_oci_is_registered() {
    let stderr = plan("[extensions]\noci.enabled = true", "oci", "");
    assert!(
        stderr.contains("invalid configuration for dependency type `oci`"),
        "{stderr}"
    );
}

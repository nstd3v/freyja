use std::{fs, process::Command};

#[test]
fn deb_switch_routes_only_when_enabled() {
    for switch in ["deb.enabled = true", "deb.enabled = false", ""] {
        let dir = std::env::temp_dir().join(format!(
            "freyja-deb-cli-{}-{}",
            std::process::id(),
            switch.len()
        ));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("Dockerfile"), "FROM scratch\n").unwrap();
        fs::write(
            dir.join("freyja.toml"),
            format!(
                r#"version = "1"
title = "Deb routing"
[extensions]
{switch}
[targets.app]
image = "example/app"
tags = ["latest"]
arches = ["amd64"]
build.context = "."
[targets.app.dependencies.package]
type = "deb"
suite = "stable"
component = "main"
arch = "amd64"
package = "nginx"
"#
            ),
        )
        .unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_freyja"))
            .args([
                "--file",
                dir.join("freyja.toml").to_str().unwrap(),
                "--state",
                dir.with_extension("state.toml").to_str().unwrap(),
                "--dir",
                dir.join("cache").to_str().unwrap(),
                "plan",
            ])
            .output()
            .unwrap();
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(!out.status.success());
        if switch.contains("true") {
            assert!(stderr.contains("unsupported Debian suite"), "{stderr}");
        } else {
            assert!(stderr.contains("unknown dependency type"), "{stderr}");
        }
        fs::remove_dir_all(dir).unwrap();
    }
}

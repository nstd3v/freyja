use super::ApkResolver;
use crate::index::{extract_index, parse_index};
use freyja_core::spec::{DependencyResolver, DependencySpec};
use freyja_core::{
    planner::{PlanAction, Planner},
    resolver::ResolverRegistry,
    spec::{Arches, BuildSpec, ExtensionsSpec, Spec, TargetSpec},
    state::State,
};
use std::collections::BTreeMap;

const INDEX: &str = "C:Q1VyOXNCoZMzZ3llFclidrdn6FHwU=\nP:nginx\nV:1.30.4-r1\nA:x86_64\nD:/bin/sh\n\nC:Q1NH6x7ZVrTqn3VGktcofFNWa+fmQ=\nP:7zip\nV:26.01-r0\nA:x86_64\n\n";

#[test]
fn parses_real_format_records_and_checksum() {
    let packages = parse_index(INDEX.as_bytes(), "x86_64").unwrap();
    let nginx = packages.get("nginx").unwrap();
    assert_eq!(nginx.version, "1.30.4-r1");
    assert_eq!(nginx.checksum, "Q1VyOXNCoZMzZ3llFclidrdn6FHwU=");
}

#[test]
fn rejects_duplicate_package_records() {
    let record = INDEX.split("\n\n").next().unwrap();
    let duplicate = format!("{record}\n\n{record}\n\n");
    assert!(parse_index(duplicate.as_bytes(), "x86_64").is_err());
}

#[test]
fn rejects_malformed_package_record() {
    assert!(parse_index(b"P:nginx\nA:x86_64\n\n", "x86_64").is_err());
}

#[test]
fn extracts_index_not_signature_or_description() {
    let mut tar = tar::Builder::new(Vec::new());
    for (name, content) in [
        (".SIGN.RSA.test", "sig"),
        ("DESCRIPTION", "desc"),
        ("APKINDEX", INDEX),
    ] {
        let mut header = tar::Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(&mut header, name, content.as_bytes())
            .unwrap();
    }
    let archive = tar.into_inner().unwrap();
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    use std::io::Write;
    encoder.write_all(&archive).unwrap();
    let bytes = encoder.finish().unwrap();
    assert_eq!(extract_index(&bytes).unwrap(), INDEX.as_bytes());
}

#[test]
fn extracts_index_across_concatenated_gzip_members() {
    // The first member is from Alpine's official v3.24/main/x86_64 index.
    let mut archive = include_bytes!("fixtures/alpine-signature-member.gz").to_vec();
    archive.extend(compressed_index(INDEX));
    assert_eq!(extract_index(&archive).unwrap(), INDEX.as_bytes());
}

fn compressed_index(index: &str) -> Vec<u8> {
    let mut tar = tar::Builder::new(Vec::new());
    let mut header = tar::Header::new_gnu();
    header.set_size(index.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar.append_data(&mut header, "APKINDEX", index.as_bytes())
        .unwrap();
    let archive = tar.into_inner().unwrap();
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    use std::io::Write;
    encoder.write_all(&archive).unwrap();
    encoder.finish().unwrap()
}

fn dependency(package: &str) -> DependencySpec {
    let mut config = toml::Table::new();
    for (key, value) in [
        ("release", "v3.24"),
        ("repository", "main"),
        ("arch", "x86_64"),
        ("package", package),
    ] {
        config.insert(key.into(), value.into());
    }
    DependencySpec {
        kind: "apk".into(),
        config,
    }
}

#[tokio::test]
async fn cached_index_resolves_version_and_changed_checksum() {
    let root = std::env::temp_dir().join(format!("freyja-apk-test-{}", std::process::id()));
    let path = root.join("v3.24/main/x86_64/APKINDEX.tar.gz");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, compressed_index(INDEX)).unwrap();
    let first = ApkResolver::new(&root)
        .resolve(&dependency("nginx"))
        .await
        .unwrap();
    assert_eq!(first.reference, "v3.24/main/x86_64/nginx");
    assert_eq!(first.metadata["version"].as_str(), Some("1.30.4-r1"));
    let changed = INDEX.replace("Q1VyOXNCoZMzZ3llFclidrdn6FHwU=", "Q1different=");
    std::fs::write(&path, compressed_index(&changed)).unwrap();
    let second = ApkResolver::new(&root)
        .resolve(&dependency("nginx"))
        .await
        .unwrap();
    assert_ne!(first.fingerprint, second.fingerprint);
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn absent_package_is_an_error_not_a_skip() {
    let root = std::env::temp_dir().join(format!("freyja-apk-missing-{}", std::process::id()));
    let path = root.join("v3.24/main/x86_64/APKINDEX.tar.gz");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, compressed_index(INDEX)).unwrap();
    let error = ApkResolver::new(&root)
        .resolve(&dependency("missing"))
        .await
        .unwrap_err();
    assert!(
        error.to_string().contains("package `missing` not found"),
        "{error}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn changed_apk_index_changes_plan_after_recorded_build() {
    let root = std::env::temp_dir().join(format!("freyja-apk-plan-{}", std::process::id()));
    let cache = root.join("cache/v3.24/main/x86_64/APKINDEX.tar.gz");
    let context = root.join("context");
    std::fs::create_dir_all(cache.parent().unwrap()).unwrap();
    std::fs::create_dir_all(&context).unwrap();
    std::fs::write(context.join("Dockerfile"), "FROM scratch\n").unwrap();
    std::fs::write(&cache, compressed_index(INDEX)).unwrap();
    let spec = Spec {
        version: "1".into(),
        title: "test".into(),
        extensions: ExtensionsSpec::default(),
        targets: BTreeMap::from([(
            "app".into(),
            TargetSpec {
                image: "example/app".into(),
                tags: vec!["latest".into()],
                arches: vec![Arches::Amd64],
                build: BuildSpec {
                    context,
                    dockerfile: "Dockerfile".into(),
                },
                dependencies: BTreeMap::from([("nginx".into(), dependency("nginx"))]),
            },
        )]),
    };
    let mut registry = ResolverRegistry::new();
    registry.register(ApkResolver::new(root.join("cache")));
    let mut state = State::default();
    let first = Planner::new(&registry).plan(&spec, &state).await.unwrap();
    assert_eq!(first.targets[0].action, PlanAction::Build);
    state.record_build(
        "app".into(),
        first.targets[0].dependencies.clone(),
        first.targets[0].build_fingerprint.clone(),
    );
    assert_eq!(
        Planner::new(&registry)
            .plan(&spec, &state)
            .await
            .unwrap()
            .targets[0]
            .action,
        PlanAction::Skip
    );

    std::fs::write(
        &cache,
        compressed_index(&INDEX.replace("1.30.4-r1", "1.30.5-r0")),
    )
    .unwrap();
    let mut refreshed = ResolverRegistry::new();
    refreshed.register(ApkResolver::new(root.join("cache")));
    assert_eq!(
        Planner::new(&refreshed)
            .plan(&spec, &state)
            .await
            .unwrap()
            .targets[0]
            .action,
        PlanAction::Build
    );
    std::fs::remove_dir_all(root).unwrap();
}

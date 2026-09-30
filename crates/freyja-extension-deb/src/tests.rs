use super::{
    cache::{IndexCache, Transport},
    dependency::DebDependency,
    index::{parse_packages, parse_release, verify_index},
    trust::{trusted_signature_status, verify_inrelease},
};
use crate::error::DebError;
use sha2::Digest;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

#[test]
fn validates_closed_source_contract() {
    let mut dep = DebDependency {
        suite: "bookworm".into(),
        component: "main".into(),
        arch: "amd64".into(),
        package: "nginx".into(),
    };
    assert!(dep.validate().is_ok());
    for bad in ["stable", "../bookworm", "bookworm-backports"] {
        dep.suite = bad.into();
        assert!(dep.validate().is_err());
    }
    dep.suite = "bookworm".into();
    for bad in ["../nginx", "", "Nginx"] {
        dep.package = bad.into();
        assert!(dep.validate().is_err());
    }
}

#[test]
fn release_exact_path_and_index_integrity() {
    let bytes = b"index";
    let sha = sha2::Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let release = format!(
        "Codename: bookworm\nComponents: main\nArchitectures: amd64 arm64\nDate: Wed, 30 Sep 2026 00:00:00 UTC\nSHA256:\n {sha} 5 main/binary-amd64/Packages.xz\n"
    );
    let entry = parse_release(&release, "bookworm", "main", "amd64", 1790726400).unwrap();
    verify_index(bytes, &entry).unwrap();
    assert!(verify_index(b"tampered", &entry).is_err());
    assert!(parse_release(&release, "bookworm-security", "main", "amd64", 1790726400).is_err());
    let updates = release.replace("Codename: bookworm", "Codename: bookworm-updates");
    assert!(parse_release(&updates, "bookworm-updates", "main", "amd64", 1790726400).is_ok());
    let security = release
        .replace("Codename: bookworm", "Codename: bookworm-security")
        .replace("Components: main", "Components: updates/main")
        + "Valid-Until: Sat, 30 Sep 2028 00:00:00 UTC\n";
    assert!(parse_release(&security, "bookworm-security", "main", "amd64", 1790726400).is_ok());
    assert!(
        parse_release(
            &release.replace(" 5 ", " 6 "),
            "bookworm",
            "main",
            "amd64",
            1790726400
        )
        .is_ok()
    );
    assert!(
        parse_release(
            &release.replace(" 5 main/", " 67108865 main/"),
            "bookworm",
            "main",
            "amd64",
            1790726400
        )
        .is_err()
    );
    assert!(
        parse_release(
            &release.replace(
                "main/binary-amd64/Packages.xz",
                "main/binary-amd64/Packages.gz"
            ),
            "bookworm",
            "main",
            "amd64",
            1790726400
        )
        .is_err()
    );
    assert!(
        parse_release(
            &release.replace("Codename: bookworm", "Codename: trixie"),
            "bookworm",
            "main",
            "amd64",
            1790726400
        )
        .is_err()
    );
    assert!(
        parse_release(
            &format!("{release}Valid-Until: Tue, 29 Sep 2026 00:00:00 UTC\n"),
            "bookworm",
            "main",
            "amd64",
            1790726400
        )
        .is_err()
    );
}

#[test]
fn package_parser_rejects_ambiguous_and_accepts_all_and_epoch() {
    let sha = "a".repeat(64);
    let record = format!(
        "Package: nginx\nVersion: 1:1.24.0-2+deb12u3\nArchitecture: all\nDescription: example\n continued\nSHA256: {sha}\n\n"
    );
    let p = parse_packages(record.as_bytes(), "amd64", "nginx").unwrap();
    assert_eq!(p["nginx"].version, "1:1.24.0-2+deb12u3");
    assert!(parse_packages(format!("{record}{record}").as_bytes(), "amd64", "nginx").is_err());
    let other = record.replace("Package: nginx", "Package: apache2");
    assert!(
        parse_packages(
            format!("{other}{other}{record}").as_bytes(),
            "amd64",
            "nginx"
        )
        .is_ok()
    );
    assert!(
        parse_packages(
            record
                .replace("Architecture: all", "Architecture: i386")
                .as_bytes(),
            "amd64",
            "nginx"
        )
        .is_err()
    );
    assert!(
        parse_packages(
            record
                .replace("SHA256:", "SHA256: nope\nSHA256:")
                .as_bytes(),
            "amd64",
            "nginx"
        )
        .is_err()
    );
    assert!(parse_packages(&[0xff], "amd64", "nginx").is_err());
}

#[test]
fn official_bookworm_fixture_verifies_only_with_archive_key() {
    let bytes = include_bytes!("../keys/bookworm-InRelease.fixture");
    let payload = verify_inrelease(bytes, "bookworm", None).unwrap();
    assert!(payload.contains("Codename: bookworm"));
    assert!(verify_inrelease(bytes, "bookworm", Some(b"wrong key")).is_err());
    let mut tampered = bytes.to_vec();
    let offset = tampered
        .windows(18)
        .position(|w| w == b"Codename: bookworm")
        .unwrap();
    tampered[offset + 10] = b'X';
    assert!(verify_inrelease(&tampered, "bookworm", None).is_err());
}

struct Fake(Mutex<VecDeque<Vec<u8>>>);
impl Transport for Fake {
    fn get<'a>(&'a self, _url: &'a str, _max: usize) -> super::cache::FetchFuture<'a> {
        Box::pin(async move {
            self.0
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| DebError::Invalid("offline".into()))
        })
    }
}
#[tokio::test]
async fn verified_cache_never_accepts_tampered_index_and_refreshes() {
    let root = std::env::temp_dir().join(format!("freyja-deb-cache-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let release1 = include_bytes!("../keys/fixture-1.InRelease").to_vec();
    let index1 = include_bytes!("../keys/fixture-1.xz").to_vec();
    let release2 = include_bytes!("../keys/fixture-2.InRelease").to_vec();
    let index2 = include_bytes!("../keys/fixture-2.xz").to_vec();
    let transport = Arc::new(Fake(Mutex::new(VecDeque::from([
        release1.clone(),
        index1.clone(),
    ]))));
    let cache = IndexCache::with_transport(
        root.clone(),
        transport.clone(),
        Arc::new(|| 1790726400),
        include_bytes!("../keys/fixture.gpg"),
    );
    let dep = DebDependency {
        suite: "bookworm".into(),
        component: "main".into(),
        arch: "amd64".into(),
        package: "nginx".into(),
    };
    assert_eq!(
        cache.packages(&dep).await.unwrap()["nginx"].version,
        "1:1.24-1"
    );
    assert_eq!(
        cache.packages(&dep).await.unwrap()["nginx"].version,
        "1:1.24-1"
    );
    let path = root.join("bookworm/main/amd64/InRelease");
    std::fs::write(&path, b"corrupt").unwrap();
    assert!(cache.packages(&dep).await.is_err());
    transport.0.lock().unwrap().extend([release2, index2]);
    assert_eq!(
        cache.packages(&dep).await.unwrap()["nginx"].version,
        "1:1.24-2"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn planner_build_skip_build_on_signed_version_change() {
    use freyja_core::{
        planner::{PlanAction, Planner},
        resolver::ResolverRegistry,
        spec::Spec,
        state::State,
    };
    let root = std::env::temp_dir().join(format!("freyja-deb-planner-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("Dockerfile"), "FROM scratch\n").unwrap();
    let spec: Spec = toml::from_str(&format!(
        r#"version = "1"
title = "test"
[extensions]
deb.enabled = true
[targets.nginx]
image = "test"
tags = ["latest"]
arches = ["amd64"]
build.context = "{}"
[targets.nginx.dependencies.package]
type = "deb"
suite = "bookworm"
component = "main"
arch = "amd64"
package = "nginx"
"#,
        root.display()
    ))
    .unwrap();
    let transport = Arc::new(Fake(Mutex::new(VecDeque::from([
        include_bytes!("../keys/fixture-1.InRelease").to_vec(),
        include_bytes!("../keys/fixture-1.xz").to_vec(),
    ]))));
    let resolver = super::DebResolver {
        cache: IndexCache::with_transport(
            root.with_extension("cache"),
            transport.clone(),
            Arc::new(|| 1790726400),
            include_bytes!("../keys/fixture.gpg"),
        ),
    };
    let mut registry = ResolverRegistry::new();
    registry.register(resolver);
    let planner = Planner::new(&registry);
    let mut state = State::default();
    let first = planner.plan(&spec, &state).await.unwrap().targets.remove(0);
    assert_eq!(first.action, PlanAction::Build);
    state.record_build(first.target, first.dependencies, first.build_fingerprint);
    assert_eq!(
        planner.plan(&spec, &state).await.unwrap().targets[0].action,
        PlanAction::Skip
    );
    transport.0.lock().unwrap().extend([
        include_bytes!("../keys/fixture-2.InRelease").to_vec(),
        include_bytes!("../keys/fixture-2.xz").to_vec(),
    ]);
    std::fs::remove_file(
        root.with_extension("cache")
            .join("bookworm/main/amd64/InRelease"),
    )
    .unwrap();
    let changed = planner.plan(&spec, &state).await.unwrap().targets.remove(0);
    assert_eq!(changed.action, PlanAction::Build);
    assert!(
        changed.dependencies["package"]
            .fingerprint
            .contains("1:1.24-2")
    );
    std::fs::remove_dir_all(&root).unwrap();
    std::fs::remove_dir_all(root.with_extension("cache")).unwrap();
}

#[tokio::test]
async fn fingerprint_includes_artifact_hash_source_and_architecture() {
    use freyja_core::spec::{DependencyResolver, DependencySpec};
    let root = std::env::temp_dir().join(format!("freyja-deb-identity-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let mut fingerprints = Vec::new();
    for (label, suite, arch) in [
        ("2", "bookworm", "amd64"),
        ("hash", "bookworm", "amd64"),
        ("source", "bookworm-updates", "amd64"),
        ("arch", "bookworm", "arm64"),
    ] {
        let release = std::fs::read(format!(
            "{}/keys/fixture-{label}.InRelease",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let index = std::fs::read(format!(
            "{}/keys/fixture-{label}.xz",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let fake = Arc::new(Fake(Mutex::new(VecDeque::from([release, index]))));
        let resolver = super::DebResolver {
            cache: IndexCache::with_transport(
                root.join(label),
                fake,
                Arc::new(|| 1790726400),
                include_bytes!("../keys/fixture.gpg"),
            ),
        };
        let dep: DependencySpec = toml::from_str(&format!("type = \"deb\"\nsuite = \"{suite}\"\ncomponent = \"main\"\narch = \"{arch}\"\npackage = \"nginx\"\n")).unwrap();
        fingerprints.push(resolver.resolve(&dep).await.unwrap().fingerprint);
    }
    assert!(fingerprints.windows(2).all(|pair| pair[0] != pair[1]));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_signature_fails_closed() {
    assert!(verify_inrelease(b"not signed", "bookworm", None).is_err());
}

#[test]
fn expired_or_revoked_signature_is_not_accepted_even_with_validsig() {
    assert!(trusted_signature_status("[GNUPG:] VALIDSIG abc\n"));
    for bad in [
        "EXPKEYSIG",
        "REVKEYSIG",
        "EXPSIG",
        "KEYREVOKED",
        "KEYEXPIRED",
        "SIGEXPIRED",
        "BADSIG",
    ] {
        assert!(!trusted_signature_status(&format!(
            "[GNUPG:] VALIDSIG abc\n[GNUPG:] {bad} abc\n"
        )));
    }
}

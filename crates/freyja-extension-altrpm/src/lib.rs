mod consts;
mod error;
mod helpers;
mod models;
mod resolver;

pub use resolver::AltRpmResolver;

#[cfg(test)]
mod tests {
    use crate::consts::*;
    use crate::helpers::{parse_header, parse_packages};
    use crate::{AltRpmResolver, models::RepositoryTransport};
    use freyja_core::spec::{DependencyResolver, DependencySpec};
    use std::io::Write;

    fn string_entry(tag: u32, offset: usize) -> [u8; 16] {
        let mut result = [0u8; 16];

        result[0..4].copy_from_slice(&tag.to_be_bytes());
        result[4..8].copy_from_slice(&RPM_STRING_TYPE.to_be_bytes());
        result[8..12].copy_from_slice(&(offset as u32).to_be_bytes());
        result[12..16].copy_from_slice(&1u32.to_be_bytes());

        result
    }

    fn u32_entry(tag: u32, offset: usize) -> [u8; 16] {
        let mut result = [0u8; 16];

        result[0..4].copy_from_slice(&tag.to_be_bytes());
        result[4..8].copy_from_slice(&RPM_INT32_TYPE.to_be_bytes());
        result[8..12].copy_from_slice(&(offset as u32).to_be_bytes());
        result[12..16].copy_from_slice(&1u32.to_be_bytes());

        result
    }

    fn package_header(
        name: &str,
        epoch: Option<u32>,
        version: &str,
        release: &str,
        arch: &str,
    ) -> Vec<u8> {
        let mut store = Vec::new();
        let mut entries = Vec::new();

        let name_offset = store.len();
        store.extend_from_slice(name.as_bytes());
        store.push(0);
        entries.push(string_entry(RPMTAG_NAME, name_offset));

        let version_offset = store.len();
        store.extend_from_slice(version.as_bytes());
        store.push(0);
        entries.push(string_entry(RPMTAG_VERSION, version_offset));

        let release_offset = store.len();
        store.extend_from_slice(release.as_bytes());
        store.push(0);
        entries.push(string_entry(RPMTAG_RELEASE, release_offset));

        if let Some(epoch) = epoch {
            while store.len() % 4 != 0 {
                store.push(0);
            }

            let epoch_offset = store.len();
            store.extend_from_slice(&epoch.to_be_bytes());

            entries.push(u32_entry(RPMTAG_EPOCH, epoch_offset));
        }

        let arch_offset = store.len();
        store.extend_from_slice(arch.as_bytes());
        store.push(0);
        entries.push(string_entry(RPMTAG_ARCH, arch_offset));

        let mut header = Vec::new();

        header.extend_from_slice(&RPM_HEADER_MAGIC);
        header.push(1);
        header.extend_from_slice(&[0; 4]);

        header.extend_from_slice(&(entries.len() as u32).to_be_bytes());

        header.extend_from_slice(&(store.len() as u32).to_be_bytes());

        for entry in entries {
            header.extend_from_slice(&entry);
        }

        header.extend_from_slice(&store);
        header
    }

    #[test]
    fn parses_package_header() {
        let bytes = package_header("nginx", None, "1.28.0", "alt1", "x86_64");

        let (package, _) = parse_header(&bytes).unwrap();

        assert_eq!(package.name, "nginx");
        assert_eq!(package.version, "1.28.0");
        assert_eq!(package.release, "alt1");
        assert_eq!(package.arch, "x86_64");
        assert_eq!(package.epoch, None);

        assert_eq!(package.nevra(), "nginx-1.28.0-alt1.x86_64");
    }

    #[test]
    fn parses_epoch() {
        let bytes = package_header("foo", Some(2), "1.0", "alt1", "x86_64");

        let (package, _) = parse_header(&bytes).unwrap();

        assert_eq!(package.epoch, Some(2));

        assert_eq!(package.nevra(), "foo-2:1.0-alt1.x86_64");
    }

    #[test]
    fn finds_package_in_header_list() {
        let mut pkglist = Vec::new();

        pkglist.extend(package_header("bash", None, "5.2", "alt1", "x86_64"));

        pkglist.extend(package_header("nginx", None, "1.28.0", "alt1", "x86_64"));

        let packages = parse_packages(&pkglist).unwrap();
        let package = packages.get("nginx").unwrap();

        assert_eq!(package.name, "nginx");
        assert_eq!(package.version, "1.28.0");
    }

    #[test]
    fn parses_adjacent_unpadded_rpm_headers() {
        // ALT's pkglist places the next header immediately after the store,
        // even when the store ends at a non-eight-byte offset.
        let mut first = package_header("bash", None, "5.2", "alt1", "x86_64");
        let count = u32::from_be_bytes(first[8..12].try_into().unwrap()) as usize;
        let store = u32::from_be_bytes(first[12..16].try_into().unwrap()) as usize;
        let raw_end = 16 + 16 * count + store;
        assert_ne!(raw_end % 8, 0, "fixture must have an unaligned store");
        assert_eq!(raw_end, first.len());
        first.extend(package_header("nginx", None, "1.28.0", "alt1", "x86_64"));

        let packages = parse_packages(&first).unwrap();
        assert_eq!(packages.get("nginx").unwrap().version, "1.28.0");
    }

    #[test]
    fn returns_none_for_missing_package() {
        let bytes = package_header("bash", None, "5.2", "alt1", "x86_64");

        assert!(!parse_packages(&bytes).unwrap().contains_key("nginx"));
    }

    fn dependency(repository: &str) -> DependencySpec {
        let mut config = toml::Table::new();
        config.insert("repository".into(), repository.into());
        config.insert("arch".into(), "x86_64".into());
        config.insert("package".into(), "nginx".into());
        DependencySpec {
            kind: "alt_rpm".into(),
            config,
        }
    }

    #[tokio::test]
    async fn resolves_cached_package_without_network() {
        let root =
            std::env::temp_dir().join(format!("freyja-alt-rpm-cache-{}", std::process::id()));
        let cache = root.join("sisyphus/x86_64/pkglist.classic.xz");
        std::fs::create_dir_all(cache.parent().unwrap()).unwrap();
        let mut encoder = xz2::write::XzEncoder::new(Vec::new(), 0);
        encoder
            .write_all(&package_header(
                "nginx",
                Some(2),
                "1.28.0",
                "alt1",
                "x86_64",
            ))
            .unwrap();
        std::fs::write(&cache, encoder.finish().unwrap()).unwrap();

        let resolved = AltRpmResolver::new(&root)
            .resolve(&dependency("sisyphus"))
            .await
            .unwrap();
        assert_eq!(resolved.reference, "sisyphus/x86_64/nginx");
        assert_eq!(resolved.fingerprint, "nginx-2:1.28.0-alt1.x86_64");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn unsupported_http_transport_returns_error_instead_of_panicking() {
        let resolver = AltRpmResolver::default().with_repository(
            "http-test",
            "example.invalid",
            "/packages",
            RepositoryTransport::Http,
        );
        let error = resolver
            .resolve(&dependency("http-test"))
            .await
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("HTTP transport is not implemented"),
            "{error}"
        );
    }
}

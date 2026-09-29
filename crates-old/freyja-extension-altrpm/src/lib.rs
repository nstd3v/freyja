mod consts;
mod error;
mod helpers;
mod models;
mod resolver;

pub use resolver::AltRpmResolver;

#[cfg(test)]
mod tests {
    use crate::consts::*;
    use crate::helpers::{find_package, parse_header};

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

        while header.len() % 8 != 0 {
            header.push(0);
        }

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

        let package = find_package(&pkglist, "nginx").unwrap().unwrap();

        assert_eq!(package.name, "nginx");
        assert_eq!(package.version, "1.28.0");
    }

    #[test]
    fn returns_none_for_missing_package() {
        let bytes = package_header("bash", None, "5.2", "alt1", "x86_64");

        assert!(find_package(&bytes, "nginx").unwrap().is_none());
    }
}

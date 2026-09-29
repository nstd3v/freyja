use std::{collections::BTreeMap, path::Path, time::SystemTime};

use super::{consts::*, error::AltRpmError, models::HeaderEntry, models::Package};

pub(crate) fn parse_header(input: &[u8]) -> Result<(Package, usize), AltRpmError> {
    // RPM header:
    //
    // 3 magic
    // 1 version
    // 4 reserved
    // 4 index count
    // 4 store size
    //
    // total fixed prefix: 16 bytes

    if input.len() < 16 {
        return Err(AltRpmError::InvalidPackageList(
            "truncated RPM header".to_owned(),
        ));
    }

    if input[0..3] != RPM_HEADER_MAGIC {
        return Err(AltRpmError::InvalidPackageList(format!(
            "invalid RPM header magic: {:02x} {:02x} {:02x}",
            input[0], input[1], input[2]
        )));
    }

    if input[3] != 1 {
        return Err(AltRpmError::InvalidPackageList(format!(
            "unsupported RPM header version {}",
            input[3]
        )));
    }

    let index_count = read_u32_be(&input[8..12])? as usize;
    let store_size = read_u32_be(&input[12..16])? as usize;

    let index_size = index_count
        .checked_mul(16)
        .ok_or_else(|| AltRpmError::InvalidPackageList("RPM index size overflow".to_owned()))?;

    let store_offset = 16usize
        .checked_add(index_size)
        .ok_or_else(|| AltRpmError::InvalidPackageList("RPM store offset overflow".to_owned()))?;

    let total_size = store_offset
        .checked_add(store_size)
        .ok_or_else(|| AltRpmError::InvalidPackageList("RPM header size overflow".to_owned()))?;

    if input.len() < total_size {
        return Err(AltRpmError::InvalidPackageList(format!(
            "truncated RPM header: need {total_size} bytes, have {}",
            input.len()
        )));
    }

    let store = &input[store_offset..total_size];

    let mut entries = Vec::with_capacity(index_count);

    for index in 0..index_count {
        let start = 16 + index * 16;

        entries.push(HeaderEntry {
            tag: read_u32_be(&input[start..start + 4])?,
            kind: read_u32_be(&input[start + 4..start + 8])?,
            offset: read_u32_be(&input[start + 8..start + 12])? as usize,
            count: read_u32_be(&input[start + 12..start + 16])?,
        });
    }

    let name = get_string(&entries, store, RPMTAG_NAME)?
        .ok_or_else(|| AltRpmError::InvalidPackageList("RPM header has no NAME".to_owned()))?;

    let version = get_string(&entries, store, RPMTAG_VERSION)?
        .ok_or_else(|| AltRpmError::InvalidPackageList("RPM header has no VERSION".to_owned()))?;

    let release = get_string(&entries, store, RPMTAG_RELEASE)?
        .ok_or_else(|| AltRpmError::InvalidPackageList("RPM header has no RELEASE".to_owned()))?;

    let arch = get_string(&entries, store, RPMTAG_ARCH)?.unwrap_or_else(|| "noarch".to_owned());

    let epoch = get_u32(&entries, store, RPMTAG_EPOCH)?;

    // RPM headers are padded to an eight-byte boundary in pkglist files.
    let consumed = total_size
        .checked_add(7)
        .ok_or_else(|| AltRpmError::InvalidPackageList("RPM header size overflow".to_owned()))?
        & !7;
    if input.len() < consumed {
        return Err(AltRpmError::InvalidPackageList(
            "truncated RPM header padding".to_owned(),
        ));
    }
    Ok((
        Package {
            name,
            epoch,
            version,
            release,
            arch,
        },
        consumed,
    ))
}

fn get_string(
    entries: &[HeaderEntry],
    store: &[u8],
    tag: u32,
) -> Result<Option<String>, AltRpmError> {
    let Some(entry) = entries.iter().find(|entry| entry.tag == tag) else {
        return Ok(None);
    };

    if entry.kind != RPM_STRING_TYPE {
        return Err(AltRpmError::InvalidPackageList(format!(
            "RPM tag {tag} has unexpected type {}",
            entry.kind
        )));
    }

    if entry.offset >= store.len() {
        return Err(AltRpmError::InvalidPackageList(format!(
            "RPM tag {tag} points outside data store"
        )));
    }

    let remaining = &store[entry.offset..];

    let nul = remaining
        .iter()
        .position(|byte| *byte == 0)
        .ok_or_else(|| {
            AltRpmError::InvalidPackageList(format!("RPM string tag {tag} is not NUL terminated"))
        })?;

    let value = std::str::from_utf8(&remaining[..nul])
        .map_err(|_| {
            AltRpmError::InvalidPackageList(format!("RPM string tag {tag} is not valid UTF-8"))
        })?
        .to_owned();

    Ok(Some(value))
}

fn get_u32(entries: &[HeaderEntry], store: &[u8], tag: u32) -> Result<Option<u32>, AltRpmError> {
    let Some(entry) = entries.iter().find(|entry| entry.tag == tag) else {
        return Ok(None);
    };

    if entry.kind != RPM_INT32_TYPE || entry.count == 0 {
        return Err(AltRpmError::InvalidPackageList(format!(
            "RPM tag {tag} has unexpected type/count"
        )));
    }

    let end = entry
        .offset
        .checked_add(4)
        .ok_or_else(|| AltRpmError::InvalidPackageList("RPM integer offset overflow".to_owned()))?;

    if end > store.len() {
        return Err(AltRpmError::InvalidPackageList(format!(
            "RPM tag {tag} points outside data store"
        )));
    }

    Ok(Some(read_u32_be(&store[entry.offset..end])?))
}

fn read_u32_be(bytes: &[u8]) -> Result<u32, AltRpmError> {
    let bytes: [u8; 4] = bytes
        .try_into()
        .map_err(|_| AltRpmError::InvalidPackageList("truncated u32".to_owned()))?;

    Ok(u32::from_be_bytes(bytes))
}

pub(crate) fn parse_packages(bytes: &[u8]) -> Result<BTreeMap<String, Package>, AltRpmError> {
    let mut packages = BTreeMap::new();
    let mut offset = 0;

    while offset < bytes.len() {
        let (package, consumed) = parse_header(&bytes[offset..])?;

        packages.insert(package.name.clone(), package);

        offset = offset
            .checked_add(consumed)
            .ok_or_else(|| AltRpmError::InvalidPackageList("header offset overflow".to_owned()))?;
    }

    Ok(packages)
}

pub(crate) async fn cache_is_fresh(path: &Path) -> Result<bool, AltRpmError> {
    let metadata = match tokio::fs::metadata(path).await {
        Ok(metadata) => metadata,

        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(false);
        }

        Err(source) => {
            return Err(AltRpmError::CacheIo {
                path: path.to_path_buf(),
                source,
            });
        }
    };

    let modified = metadata.modified().map_err(|source| AltRpmError::CacheIo {
        path: path.to_path_buf(),
        source,
    })?;

    let age = SystemTime::now()
        .duration_since(modified)
        .unwrap_or_default();

    Ok(age < CACHE_TTL)
}

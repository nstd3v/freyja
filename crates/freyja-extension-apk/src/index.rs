use std::{collections::BTreeMap, io::Read};

use crate::error::ApkError;

const MAX_INDEX_SIZE: u64 = 16 * 1024 * 1024;
const MAX_TAR_SIZE: u64 = 32 * 1024 * 1024;

#[derive(Debug, Clone)]
pub(crate) struct Package {
    pub version: String,
    pub checksum: String,
}

pub(crate) fn extract_index(compressed: &[u8]) -> Result<Vec<u8>, ApkError> {
    let gz = flate2::read::MultiGzDecoder::new(compressed);
    let mut archive = tar::Archive::new(gz.take(MAX_TAR_SIZE));
    let entries = archive
        .entries()
        .map_err(|e| ApkError::InvalidIndex(e.to_string()))?;
    for entry in entries {
        let mut entry = entry.map_err(|e| ApkError::InvalidIndex(e.to_string()))?;
        let path = entry
            .path()
            .map_err(|e| ApkError::InvalidIndex(e.to_string()))?;
        if path.as_ref() != std::path::Path::new("APKINDEX") {
            continue;
        }
        if !entry.header().entry_type().is_file() || entry.size() > MAX_INDEX_SIZE {
            return Err(ApkError::InvalidIndex(
                "APKINDEX is not a bounded regular file".into(),
            ));
        }
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .map_err(|e| ApkError::InvalidIndex(e.to_string()))?;
        return Ok(bytes);
    }
    Err(ApkError::InvalidIndex("APKINDEX entry missing".into()))
}

pub(crate) fn parse_index(input: &[u8], arch: &str) -> Result<BTreeMap<String, Package>, ApkError> {
    let text = std::str::from_utf8(input).map_err(|e| ApkError::InvalidIndex(e.to_string()))?;
    let mut packages = BTreeMap::new();
    for record in text.split("\n\n") {
        if record.trim().is_empty() {
            continue;
        }
        let mut name = None;
        let mut version = None;
        let mut record_arch = None;
        let mut checksum = None;
        for line in record.lines() {
            let line = line.trim_end_matches('\r');
            if line.is_empty() {
                continue;
            }
            let (key, value) = line
                .split_once(':')
                .ok_or_else(|| ApkError::InvalidIndex("invalid index field".into()))?;
            match key {
                "P" => name = Some(value),
                "V" => version = Some(value),
                "A" => record_arch = Some(value),
                "C" => checksum = Some(value),
                _ => {}
            }
        }
        let (Some(name), Some(version), Some(record_arch), Some(checksum)) =
            (name, version, record_arch, checksum)
        else {
            return Err(ApkError::InvalidIndex(
                "package missing P, V, A, or C".into(),
            ));
        };
        if name.is_empty() || version.is_empty() || checksum.is_empty() || record_arch != arch {
            return Err(ApkError::InvalidIndex(format!(
                "invalid package `{name}` for {arch}"
            )));
        }
        if packages
            .insert(
                name.to_owned(),
                Package {
                    version: version.to_owned(),
                    checksum: checksum.to_owned(),
                },
            )
            .is_some()
        {
            return Err(ApkError::InvalidIndex(format!(
                "duplicate package `{name}`"
            )));
        }
    }
    if packages.is_empty() {
        return Err(ApkError::InvalidIndex("empty package index".into()));
    }
    Ok(packages)
}

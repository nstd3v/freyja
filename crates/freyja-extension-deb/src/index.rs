use crate::error::DebError;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, io::Read};

pub(crate) struct IndexEntry {
    pub hash: String,
    pub size: usize,
}
#[derive(Clone)]
pub(crate) struct Package {
    pub version: String,
    pub sha256: String,
}
fn bad(reason: impl Into<String>) -> DebError {
    DebError::Invalid(reason.into())
}
fn hex_hash(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

pub(crate) fn parse_release(
    text: &str,
    suite: &str,
    component: &str,
    arch: &str,
    now: i64,
) -> Result<IndexEntry, DebError> {
    let path = format!("{component}/binary-{arch}/Packages.xz");
    let mut fields = BTreeMap::new();
    let mut hashes = BTreeMap::new();
    let mut in_sha = false;
    for line in text.lines() {
        if line.starts_with(' ') && in_sha {
            let cols: Vec<_> = line.split_whitespace().collect();
            if cols.len() != 3 || !hex_hash(cols[0]) {
                return Err(bad("malformed Release SHA256 entry"));
            }
            let size = cols[1]
                .parse::<usize>()
                .map_err(|_| bad("invalid index size"))?;
            if hashes.insert(cols[2], (cols[0], size)).is_some() {
                return Err(bad("duplicate Release SHA256 path"));
            }
        } else if !line.starts_with(' ') {
            let (key, value) = line
                .split_once(':')
                .ok_or_else(|| bad("invalid Release field"))?;
            if fields.insert(key, value.trim()).is_some() {
                return Err(bad("duplicate Release field"));
            }
            in_sha = key == "SHA256";
        }
    }
    if fields.get("Codename") != Some(&suite) {
        return Err(bad("Release codename mismatch"));
    }
    let signed_component = if suite == "bookworm-security" {
        "updates/main"
    } else {
        component
    };
    if !fields
        .get("Components")
        .is_some_and(|s| s.split_whitespace().any(|x| x == signed_component))
    {
        return Err(bad("Release component missing"));
    }
    if !fields
        .get("Architectures")
        .is_some_and(|s| s.split_whitespace().any(|x| x == arch))
    {
        return Err(bad("Release architecture missing"));
    }
    let date = parse_date(
        fields
            .get("Date")
            .ok_or_else(|| bad("Release Date missing"))?,
    )?;
    if date > now + 86400 {
        return Err(bad("Release Date is in the future"));
    }
    match fields.get("Valid-Until") {
        Some(end) if parse_date(end)? < now => return Err(bad("Release expired")),
        _ => {}
    }
    let (hash, size) = hashes
        .get(path.as_str())
        .ok_or_else(|| bad("exact Packages.xz entry missing"))?;
    if *size > 64 * 1024 * 1024 {
        return Err(bad("compressed index exceeds limit"));
    }
    Ok(IndexEntry {
        hash: hash.to_ascii_lowercase(),
        size: *size,
    })
}
fn parse_date(s: &str) -> Result<i64, DebError> {
    let normalized = s
        .strip_suffix(" UTC")
        .map(|v| format!("{v} +0000"))
        .unwrap_or_else(|| s.to_owned());
    chrono::DateTime::parse_from_rfc2822(&normalized)
        .map(|d| d.timestamp())
        .map_err(|_| bad("invalid Release date"))
}
pub(crate) fn verify_index(bytes: &[u8], entry: &IndexEntry) -> Result<(), DebError> {
    if bytes.len() != entry.size
        || Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
            != entry.hash
    {
        return Err(bad("Packages.xz size or SHA256 mismatch"));
    }
    Ok(())
}
pub(crate) fn decompress(bytes: &[u8]) -> Result<Vec<u8>, DebError> {
    let decoder = xz2::read::XzDecoder::new(bytes);
    let mut result = Vec::new();
    decoder
        .take(256 * 1024 * 1024 + 1)
        .read_to_end(&mut result)
        .map_err(|e| bad(format!("XZ decoding: {e}")))?;
    if result.len() > 256 * 1024 * 1024 {
        return Err(bad("expanded index exceeds limit"));
    }
    Ok(result)
}
pub(crate) fn parse_packages(
    bytes: &[u8],
    arch: &str,
    target: &str,
) -> Result<BTreeMap<String, Package>, DebError> {
    let text = std::str::from_utf8(bytes).map_err(|_| bad("invalid UTF-8 in Packages"))?;
    let mut packages = BTreeMap::new();
    let mut fields: BTreeMap<&str, &str> = BTreeMap::new();
    let mut count = 0usize;
    let mut commit = |fields: &mut BTreeMap<&str, &str>| -> Result<(), DebError> {
        if fields.is_empty() {
            return Ok(());
        }
        count += 1;
        if count > 500_000 {
            return Err(bad("too many package records"));
        }
        let get = |key| {
            fields
                .get(key)
                .copied()
                .filter(|v| !v.is_empty())
                .ok_or_else(|| bad(format!("missing {key}")))
        };
        let name = get("Package")?;
        let version = get("Version")?;
        let record_arch = get("Architecture")?;
        let hash = get("SHA256")?;
        if record_arch != arch && record_arch != "all" {
            return Err(bad("package architecture mismatch"));
        }
        if name.is_empty()
            || !name.bytes().all(|b| {
                b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'+' | b'-' | b'.')
            })
            || !hex_hash(hash)
            || version.contains(char::is_whitespace)
        {
            return Err(bad("invalid package identity/version/SHA256"));
        }
        if name == target
            && packages
                .insert(
                    name.into(),
                    Package {
                        version: version.into(),
                        sha256: hash.into(),
                    },
                )
                .is_some()
        {
            return Err(bad(format!("duplicate target package `{name}`")));
        }
        fields.clear();
        Ok(())
    };
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            commit(&mut fields)?;
            continue;
        }
        if line.starts_with(' ') || line.starts_with('\t') {
            if fields.is_empty() {
                return Err(bad("orphan continuation"));
            }
            continue;
        }
        let (key, value) = line
            .split_once(": ")
            .ok_or_else(|| bad("malformed control field"))?;
        if key.is_empty()
            || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || fields.insert(key, value).is_some()
        {
            return Err(bad("duplicate or malformed control field"));
        }
    }
    commit(&mut fields)?;
    if count == 0 {
        return Err(bad("empty package index"));
    }
    Ok(packages)
}

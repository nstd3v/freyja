use crate::error::DebError;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);

// These binary keyrings are vendored from ftp-master.debian.org/keys, never from fetched metadata.
const ARCHIVE: &[u8] = include_bytes!("../keys/archive-key-12.gpg");
const SECURITY: &[u8] = include_bytes!("../keys/archive-key-12-security.gpg");

pub(crate) fn verify_inrelease(
    bytes: &[u8],
    suite: &str,
    key_override: Option<&[u8]>,
) -> Result<String, DebError> {
    if bytes.len() > 4 * 1024 * 1024 {
        return Err(DebError::Invalid("InRelease exceeds limit".into()));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| DebError::Invalid("InRelease is not UTF-8".into()))?;
    if !text.starts_with("-----BEGIN PGP SIGNED MESSAGE-----\n")
        || !text.contains("\n-----BEGIN PGP SIGNATURE-----\n")
    {
        return Err(DebError::Invalid("missing clear-signed InRelease".into()));
    }
    let dir = std::env::temp_dir().join(format!(
        "freyja-deb-gpgv-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&dir)?;
    let result = (|| {
        let key = dir.join("trusted.gpg");
        let signed = dir.join("InRelease");
        write_exclusive(
            &key,
            key_override.unwrap_or(if suite == "bookworm-security" {
                SECURITY
            } else {
                ARCHIVE
            }),
        )?;
        write_exclusive(&signed, bytes)?;
        let out = Command::new("gpgv")
            .arg("--status-fd")
            .arg("1")
            .arg("--keyring")
            .arg(&key)
            .arg("--")
            .arg(&signed)
            .output()?;
        let status = String::from_utf8_lossy(&out.stdout);
        // Debian InRelease can carry additional signatures from newer keys/algorithms.
        // Require at least one verified signature from this suite's pinned keyring;
        // unknown additional signers are not trust anchors.
        if !trusted_signature_status(&status) {
            return Err(DebError::Invalid(format!(
                "InRelease signature verification failed: {}",
                String::from_utf8_lossy(&out.stderr)
            )));
        }
        let content = text
            .split_once("\n\n")
            .ok_or_else(|| DebError::Invalid("missing signed payload".into()))?
            .1;
        let payload = content
            .split_once("\n-----BEGIN PGP SIGNATURE-----\n")
            .ok_or_else(|| DebError::Invalid("missing signature boundary".into()))?
            .0;
        let mut unescaped = String::new();
        for line in payload.lines() {
            unescaped.push_str(line.strip_prefix("- ").unwrap_or(line));
            unescaped.push('\n');
        }
        Ok(unescaped)
    })();
    let _ = fs::remove_dir_all(dir);
    result
}
pub(crate) fn trusted_signature_status(status: &str) -> bool {
    let mut valid = false;
    for line in status.lines() {
        let Some(event) = line
            .strip_prefix("[GNUPG:] ")
            .and_then(|s| s.split_whitespace().next())
        else {
            continue;
        };
        match event {
            "VALIDSIG" => valid = true,
            "BADSIG" | "EXPKEYSIG" | "REVKEYSIG" | "EXPSIG" | "KEYREVOKED" | "KEYEXPIRED"
            | "SIGEXPIRED" => return false,
            _ => {}
        }
    }
    valid
}
fn write_exclusive(path: &PathBuf, bytes: &[u8]) -> Result<(), DebError> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    Ok(())
}

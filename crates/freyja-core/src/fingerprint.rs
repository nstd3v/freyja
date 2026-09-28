use std::{fmt::Write, fs, io::Read, path::{Component, Path}};

use sha2::{Digest, Sha256};

use crate::{error::Error, spec::TargetSpec};

/// Conservative fingerprint: include every file in the context, even files
/// excluded by container ignore rules. This may rebuild too often, but cannot
/// silently miss an input. Generated state must live outside the context.
pub fn fingerprint(target: &TargetSpec) -> Result<String, Error> {
    let context = &target.build.context;
    let metadata = fs::symlink_metadata(context).map_err(|source| Error::BuildInput {
        path: context.clone(), source,
    })?;
    if !metadata.is_dir() {
        return Err(Error::InvalidBuildInput {
            path: context.clone(),
            reason: "build context must be a directory".into(),
        });
    }

    let dockerfile = context.join(&target.build.dockerfile);
    if target.build.dockerfile.as_os_str().is_empty()
        || !target.build.dockerfile.components().all(|part| matches!(part, Component::Normal(_)))
        || !dockerfile.is_file()
    {
        return Err(Error::InvalidBuildInput {
            path: dockerfile,
            reason: "Dockerfile is missing or outside the build context".into(),
        });
    }

    let mut hash = Sha256::new();
    hash.update(b"freyja-build-input-v1\0");
    let spec = toml::to_string(target).map_err(|source| Error::SerializeBuildInput { source })?;
    field(&mut hash, spec.as_bytes());
    visit(context, context, &mut hash)?;
    let mut result = String::with_capacity(64);
    for byte in hash.finalize() {
        write!(result, "{byte:02x}").expect("writing to a String cannot fail");
    }
    Ok(format!("v1:{result}"))
}

fn field(hash: &mut Sha256, bytes: &[u8]) {
    hash.update((bytes.len() as u64).to_le_bytes());
    hash.update(bytes);
}

fn visit(root: &Path, dir: &Path, hash: &mut Sha256) -> Result<(), Error> {
    let mut entries = fs::read_dir(dir)
        .map_err(|source| Error::BuildInput { path: dir.into(), source })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| Error::BuildInput { path: dir.into(), source })?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let relative = path.strip_prefix(root).expect("entry within context");
        let name = relative.to_str().ok_or_else(|| Error::InvalidBuildInput {
            path: path.clone(),
            reason: "non-UTF-8 paths are not supported".into(),
        })?;
        let metadata = fs::symlink_metadata(&path).map_err(|source| Error::BuildInput {
            path: path.clone(), source,
        })?;
        if metadata.is_dir() {
            field(hash, b"dir");
            field(hash, name.as_bytes());
            visit(root, &path, hash)?;
        } else if metadata.is_file() {
            field(hash, b"file");
            field(hash, name.as_bytes());
            let mut file = fs::File::open(&path).map_err(|source| Error::BuildInput {
                path: path.clone(), source,
            })?;
            field(hash, &metadata.len().to_le_bytes());
            let mut buffer = [0u8; 8192];
            loop {
                let count = file.read(&mut buffer).map_err(|source| Error::BuildInput {
                    path: path.clone(), source,
                })?;
                if count == 0 { break; }
                hash.update(&buffer[..count]);
            }
        } else {
            return Err(Error::InvalidBuildInput {
                path,
                reason: "symlinks and special files are not supported in build contexts".into(),
            });
        }
    }
    Ok(())
}

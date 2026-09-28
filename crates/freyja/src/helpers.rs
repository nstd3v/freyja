use std::path::{Component, Path, PathBuf};

use freyja_core::{error::Error, spec::Spec, state::State};

pub fn load_spec(path: &Path) -> Result<Spec, Error> {
    let contents = std::fs::read_to_string(path).map_err(|source| Error::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;

    let mut spec: Spec = toml::from_str(&contents).map_err(|source| Error::ParseConfig {
        path: path.to_path_buf(),
        source,
    })?;
    let file = std::fs::canonicalize(path).map_err(|source| Error::BuildInput {
        path: path.to_path_buf(),
        source,
    })?;
    let base = file.parent().expect("canonical file has a parent");
    for target in spec.targets.values_mut() {
        let build = &mut target.build;
        let context = if build.context.is_absolute() {
            build.context.clone()
        } else {
            base.join(&build.context)
        };
        // Do not follow a symlink used as the context root.
        let metadata = std::fs::symlink_metadata(&context).map_err(|source| Error::BuildInput {
            path: context.clone(),
            source,
        })?;
        if !metadata.is_dir() {
            return Err(Error::InvalidBuildInput {
                path: context,
                reason: "build context must be a directory, not a symlink".into(),
            });
        }
        let context = std::fs::canonicalize(&context).map_err(|source| Error::BuildInput {
            path: context.clone(),
            source,
        })?;
        let dockerfile = &build.dockerfile;
        if dockerfile.as_os_str().is_empty()
            || !dockerfile
                .components()
                .all(|part| matches!(part, Component::Normal(_)))
        {
            return Err(Error::InvalidBuildInput {
                path: dockerfile.clone(),
                reason: "Dockerfile must be relative to the context".into(),
            });
        }
        let file_path = context.join(dockerfile);
        let metadata =
            std::fs::symlink_metadata(&file_path).map_err(|source| Error::BuildInput {
                path: file_path.clone(),
                source,
            })?;
        let resolved = std::fs::canonicalize(&file_path).map_err(|source| Error::BuildInput {
            path: file_path.clone(),
            source,
        })?;
        if !metadata.is_file() || !resolved.starts_with(&context) {
            return Err(Error::InvalidBuildInput {
                path: file_path,
                reason: "Dockerfile must be a regular file inside the context".into(),
            });
        }
        build.context = context;
    }
    Ok(spec)
}

pub fn load_state(path: &Path) -> Result<State, Error> {
    if !path.exists() {
        return Ok(State::default());
    }

    let contents = std::fs::read_to_string(path).map_err(|source| Error::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;

    toml::from_str(&contents).map_err(|source| Error::ParseState {
        path: path.to_path_buf(),
        source,
    })
}

/// State inside a build context would change the context hash every time it is saved.
pub fn validate_state_path(spec: &Spec, state_path: &Path) -> Result<(), Error> {
    let state = canonical_with_missing_suffix(state_path)?;
    for target in spec.targets.values() {
        let context =
            std::fs::canonicalize(&target.build.context).map_err(|source| Error::BuildInput {
                path: target.build.context.clone(),
                source,
            })?;
        if state.starts_with(&context) {
            return Err(Error::InvalidBuildInput {
                path: state_path.into(),
                reason: format!(
                    "state is inside build context `{}`; move it outside to avoid a rebuild loop",
                    context.display()
                ),
            });
        }
    }
    Ok(())
}

fn canonical_with_missing_suffix(path: &Path) -> Result<PathBuf, Error> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|source| Error::BuildInput {
                path: path.into(),
                source,
            })?
            .join(path)
    };
    let mut ancestor = absolute.as_path();
    while !ancestor.exists() {
        ancestor = ancestor.parent().ok_or_else(|| Error::InvalidBuildInput {
            path: path.into(),
            reason: "state path has no existing ancestor".into(),
        })?;
    }
    let mut resolved = std::fs::canonicalize(ancestor).map_err(|source| Error::BuildInput {
        path: ancestor.into(),
        source,
    })?;
    for component in absolute.strip_prefix(ancestor).unwrap().components() {
        match component {
            Component::Normal(name) => resolved.push(name),
            Component::ParentDir => {
                resolved.pop();
            }
            Component::CurDir => {}
            _ => {
                return Err(Error::InvalidBuildInput {
                    path: path.into(),
                    reason: "invalid state path".into(),
                });
            }
        }
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests;

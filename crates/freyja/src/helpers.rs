use std::path::{Component, Path, PathBuf};

use freyja_core::{error::Error, spec::Spec, state::State};

pub fn load_spec(path: &Path) -> Result<Spec, Error> {
    let contents = std::fs::read_to_string(path).map_err(|source| Error::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;

    toml::from_str(&contents).map_err(|source| Error::ParseConfig {
        path: path.to_path_buf(),
        source,
    })
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
        let context = std::fs::canonicalize(&target.build.context).map_err(|source| Error::BuildInput {
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
            .map_err(|source| Error::BuildInput { path: path.into(), source })?
            .join(path)
    };
    let mut ancestor = absolute.as_path();
    while !ancestor.exists() {
        ancestor = ancestor.parent().ok_or_else(|| Error::InvalidBuildInput {
            path: path.into(), reason: "state path has no existing ancestor".into(),
        })?;
    }
    let mut resolved = std::fs::canonicalize(ancestor).map_err(|source| Error::BuildInput {
        path: ancestor.into(), source,
    })?;
    for component in absolute.strip_prefix(ancestor).unwrap().components() {
        match component {
            Component::Normal(name) => resolved.push(name),
            Component::ParentDir => { resolved.pop(); }
            Component::CurDir => {}
            _ => return Err(Error::InvalidBuildInput {
                path: path.into(), reason: "invalid state path".into(),
            }),
        }
    }
    Ok(resolved)
}

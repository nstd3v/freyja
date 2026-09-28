use std::path::Path;

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

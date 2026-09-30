#[derive(Debug, thiserror::Error)]
pub(crate) enum DebError {
    #[error("invalid Debian dependency or metadata: {0}")]
    Invalid(String),
    #[error("Debian package `{0}` not found")]
    NotFound(String),
    #[error("Debian HTTP failure: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Debian cache failure: {0}")]
    Io(#[from] std::io::Error),
}

use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum ApkError {
    #[error("invalid APK dependency: {0}")]
    InvalidConfig(String),
    #[error("unsupported APK repository `{0}`")]
    UnsupportedRepository(String),
    #[error("failed to fetch APK index: {0}")]
    Http(#[from] reqwest::Error),
    #[error("failed to access APK cache: {0}")]
    Cache(#[from] std::io::Error),
    #[error("invalid APK index: {0}")]
    InvalidIndex(String),
    #[error("APK package `{0}` not found")]
    PackageNotFound(String),
}

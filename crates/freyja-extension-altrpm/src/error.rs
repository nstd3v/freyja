use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AltRpmError {
    #[error("{transport} transport is not implemented for ALT RPM repositories")]
    UnsupportedTransport { transport: &'static str },

    #[error("unknown ALT RPM repository `{repository}`")]
    UnknownRepository { repository: String },

    #[error("FTP request to `{host}` failed: {source}")]
    Ftp {
        host: String,

        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("failed to access cache path `{}`", path.display())]
    CacheIo {
        path: PathBuf,

        #[source]
        source: std::io::Error,
    },

    #[error("failed to decompress package list: {source}")]
    Decompression {
        #[source]
        source: std::io::Error,
    },

    #[error("invalid RPM package list: {0}")]
    InvalidPackageList(String),

    #[error("package `{package}` not found in repository `{repository}` for architecture `{arch}`")]
    PackageNotFound {
        package: String,
        repository: String,
        arch: String,
    },
}

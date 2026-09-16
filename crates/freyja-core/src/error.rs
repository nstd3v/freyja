use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid configuration for dependency type `{kind}`")]
    InvalidDependencyConfig {
        kind: String,
        #[source]
        source: toml::de::Error,
    },

    #[error("unknown dependency type: {kind}")]
    UnknownDependencyType { kind: String },

    #[error("invalid `{kind}` dependency reference `{reference}`")]
    InvalidDependencyReference {
        kind: String,
        reference: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("failed to resolve `{kind}` dependency `{reference}`")]
    DependencyResolution {
        kind: String,
        reference: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

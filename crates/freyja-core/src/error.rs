use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("unknown dependency type: {kind}")]
    UnknownDependencyType { kind: String },
}

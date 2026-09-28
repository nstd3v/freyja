use crate::error::Error;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DependencySpec {
    #[serde(rename = "type")]
    pub kind: String,

    #[serde(flatten)]
    pub config: toml::Table,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResolvedDependency {
    pub kind: String,
    pub reference: String,
    pub fingerprint: String,

    #[serde(default)]
    pub metadata: toml::Table,
}

#[async_trait::async_trait]
pub trait DependencyResolver: Send + Sync {
    /// Dependency type handled by this resolver.
    fn kind(&self) -> &'static str;

    async fn resolve(&self, dependency: &DependencySpec) -> Result<ResolvedDependency, Error>;
}

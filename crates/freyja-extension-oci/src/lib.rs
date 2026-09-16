use async_trait::async_trait;
use freyja_core::spec::deps::{DependencyResolver, DependencySpec, ResolvedDependency};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct OciDependency {
    pub r#ref: String,
}

pub struct OciResolver {
    // registry client, auth config, etc.
}

#[async_trait]
impl DependencyResolver for OciResolver {
    fn kind(&self) -> &'static str {
        "oci"
    }

    async fn resolve(
        &self,
        dependency: &DependencySpec,
    ) -> Result<ResolvedDependency, freyja_core::error::Error> {
        // 1. deserialize dependency.config into OciDependency
        // 2. query OCI registry
        // 3. get manifest digest
        // 4. return canonical result

        todo!()
    }
}

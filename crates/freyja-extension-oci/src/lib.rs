use async_trait::async_trait;
use freyja_core::{
    error::Error,
    spec::deps::{DependencyResolver, DependencySpec, ResolvedDependency},
};
use oci_client::{Client, Reference, secrets::RegistryAuth};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct OciDependency {
    pub r#ref: String,
}

pub struct OciResolver {
    client: Client,
}

impl OciResolver {
    pub fn new() -> Self {
        Self {
            client: Client::new(Default::default()),
        }
    }
}

impl Default for OciResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DependencyResolver for OciResolver {
    fn kind(&self) -> &'static str {
        "oci"
    }

    async fn resolve(&self, dependency: &DependencySpec) -> Result<ResolvedDependency, Error> {
        let config: OciDependency = dependency.config.clone().try_into().map_err(|source| {
            Error::InvalidDependencyConfig {
                kind: self.kind().to_owned(),
                source,
            }
        })?;

        let reference: Reference =
            config
                .r#ref
                .parse()
                .map_err(|source| Error::InvalidDependencyReference {
                    kind: self.kind().to_owned(),
                    reference: config.r#ref.clone(),
                    source: Box::new(source),
                })?;

        // Anonymous is enough for the initial implementation.
        let auth = RegistryAuth::Anonymous;

        let digest = self
            .client
            .fetch_manifest_digest(&reference, &auth)
            .await
            .map_err(|source| Error::DependencyResolution {
                kind: self.kind().to_owned(),
                reference: config.r#ref.clone(),
                source: Box::new(source),
            })?;

        Ok(ResolvedDependency {
            kind: self.kind().to_owned(),
            reference: config.r#ref,
            fingerprint: digest,
            metadata: Default::default(),
        })
    }
}

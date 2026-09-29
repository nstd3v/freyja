mod cache;
mod dependency;
mod error;
mod index;

use freyja_core::{
    error::Error,
    spec::{DependencyResolver, DependencySpec, ResolvedDependency},
};

use crate::{cache::IndexCache, dependency::ApkDependency, error::ApkError};

pub struct ApkResolver {
    cache: IndexCache,
}

#[async_trait::async_trait]
impl DependencyResolver for ApkResolver {
    fn kind(&self) -> &'static str {
        "apk"
    }

    async fn resolve(&self, dependency: &DependencySpec) -> Result<ResolvedDependency, Error> {
        let config: ApkDependency = dependency.config.clone().try_into().map_err(|source| {
            Error::InvalidDependencyConfig {
                kind: self.kind().into(),
                source,
            }
        })?;
        let reference = config.reference();
        let resolved = async {
            config.validate()?;
            let packages = self.cache.packages(&config).await?;
            packages
                .get(&config.package)
                .cloned()
                .ok_or_else(|| ApkError::PackageNotFound(config.package.clone()))
        }
        .await
        .map_err(|source| Error::DependencyResolution {
            kind: self.kind().into(),
            reference: reference.clone(),
            source: Box::new(source),
        })?;
        let mut metadata = toml::Table::new();
        metadata.insert("version".into(), resolved.version.clone().into());
        metadata.insert("checksum".into(), resolved.checksum.clone().into());
        Ok(ResolvedDependency {
            kind: self.kind().into(),
            reference: reference.clone(),
            fingerprint: format!("{reference}@{}#{}", resolved.version, resolved.checksum),
            metadata,
        })
    }
}

#[cfg(test)]
mod tests;

mod cache;
mod dependency;
mod error;
mod index;
mod trust;

use cache::IndexCache;
use dependency::DebDependency;
use error::DebError;
use freyja_core::{
    error::Error,
    spec::{DependencyResolver, DependencySpec, ResolvedDependency},
};

pub struct DebResolver {
    cache: IndexCache,
}
impl DebResolver {
    pub fn new(path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            cache: IndexCache::new(path.into()),
        }
    }
}
#[async_trait::async_trait]
impl DependencyResolver for DebResolver {
    fn kind(&self) -> &'static str {
        "deb"
    }
    async fn resolve(&self, dependency: &DependencySpec) -> Result<ResolvedDependency, Error> {
        let config: DebDependency = dependency.config.clone().try_into().map_err(|source| {
            Error::InvalidDependencyConfig {
                kind: "deb".into(),
                source,
            }
        })?;
        let reference = config.reference();
        let resolved = async {
            config.validate()?;
            self.cache
                .packages(&config)
                .await?
                .get(&config.package)
                .cloned()
                .ok_or_else(|| DebError::NotFound(config.package.clone()))
        }
        .await
        .map_err(|source| Error::DependencyResolution {
            kind: "deb".into(),
            reference: reference.clone(),
            source: Box::new(source),
        })?;
        let mut metadata = toml::Table::new();
        for (key, val) in [
            ("version", resolved.version.as_str()),
            ("sha256", resolved.sha256.as_str()),
            ("suite", config.suite.as_str()),
            ("component", config.component.as_str()),
            ("arch", config.arch.as_str()),
        ] {
            metadata.insert(key.into(), val.into());
        }
        Ok(ResolvedDependency {
            kind: "deb".into(),
            reference: reference.clone(),
            fingerprint: format!("{reference}@{}#{}", resolved.version, resolved.sha256),
            metadata,
        })
    }
}
#[cfg(test)]
mod tests;

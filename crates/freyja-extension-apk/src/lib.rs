mod index;

use std::{
    collections::BTreeMap,
    path::PathBuf,
    time::{Duration, SystemTime},
};

use freyja_core::{
    error::Error,
    spec::{DependencyResolver, DependencySpec, ResolvedDependency},
};
use index::{ApkError, Package, extract_index, parse_index};
use serde::Deserialize;
use tokio::sync::Mutex;

const CACHE_TTL: Duration = Duration::from_secs(60 * 60);
const MAX_DOWNLOAD_SIZE: usize = 8 * 1024 * 1024;
const BASE_URL: &str = "https://dl-cdn.alpinelinux.org/alpine";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ApkDependency {
    release: String,
    repository: String,
    arch: String,
    package: String,
}

impl ApkDependency {
    fn validate(&self) -> Result<(), ApkError> {
        let numbers = self
            .release
            .strip_prefix('v')
            .and_then(|v| v.split_once('.'));
        if !matches!(numbers, Some((major, minor)) if !major.is_empty() && !minor.is_empty() && major.bytes().all(|b| b.is_ascii_digit()) && minor.bytes().all(|b| b.is_ascii_digit()))
        {
            return Err(ApkError::InvalidConfig(
                "release must be a stable version such as v3.24".into(),
            ));
        }
        if !matches!(self.repository.as_str(), "main" | "community") {
            return Err(ApkError::UnsupportedRepository(self.repository.clone()));
        }
        if !matches!(
            self.arch.as_str(),
            "x86_64" | "aarch64" | "x86" | "armv7" | "ppc64le" | "s390x" | "riscv64"
        ) {
            return Err(ApkError::InvalidConfig(format!(
                "unsupported architecture `{}`",
                self.arch
            )));
        }
        if self.package.is_empty()
            || !self
                .package
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'_' | b'.'))
        {
            return Err(ApkError::InvalidConfig("invalid package name".into()));
        }
        Ok(())
    }

    fn reference(&self) -> String {
        format!(
            "{}/{}/{}/{}",
            self.release, self.repository, self.arch, self.package
        )
    }
}

pub struct ApkResolver {
    cache_dir: PathBuf,
    client: reqwest::Client,
    indexes: Mutex<BTreeMap<(String, String, String), BTreeMap<String, Package>>>,
}

impl ApkResolver {
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            cache_dir: cache_dir.into(),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("valid HTTP client"),
            indexes: Mutex::new(BTreeMap::new()),
        }
    }

    async fn packages(
        &self,
        config: &ApkDependency,
    ) -> Result<BTreeMap<String, Package>, ApkError> {
        let key = (
            config.release.clone(),
            config.repository.clone(),
            config.arch.clone(),
        );
        if let Some(packages) = self.indexes.lock().await.get(&key) {
            return Ok(packages.clone());
        }
        let path = self
            .cache_dir
            .join(&config.release)
            .join(&config.repository)
            .join(&config.arch)
            .join("APKINDEX.tar.gz");
        let bytes = if is_fresh(&path).await? {
            tokio::fs::read(&path).await?
        } else {
            let url = format!(
                "{}/{}/{}/{}/APKINDEX.tar.gz",
                BASE_URL, config.release, config.repository, config.arch
            );
            let mut response = self.client.get(url).send().await?.error_for_status()?;
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await? {
                if bytes.len().saturating_add(chunk.len()) > MAX_DOWNLOAD_SIZE {
                    return Err(ApkError::InvalidIndex(
                        "compressed index exceeds download limit".into(),
                    ));
                }
                bytes.extend_from_slice(&chunk);
            }
            // Validate before replacing an older cache entry.
            let parsed = parse_index(&extract_index(&bytes)?, &config.arch)?;
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
            tokio::fs::write(&temporary, &bytes).await?;
            tokio::fs::rename(&temporary, &path).await?;
            self.indexes.lock().await.insert(key, parsed.clone());
            return Ok(parsed);
        };
        if bytes.len() > MAX_DOWNLOAD_SIZE {
            return Err(ApkError::InvalidIndex(
                "cached index exceeds download limit".into(),
            ));
        }
        let parsed = parse_index(&extract_index(&bytes)?, &config.arch)?;
        self.indexes.lock().await.insert(key, parsed.clone());
        Ok(parsed)
    }
}

async fn is_fresh(path: &std::path::Path) -> Result<bool, ApkError> {
    match tokio::fs::metadata(path).await {
        Ok(metadata) => Ok(SystemTime::now()
            .duration_since(metadata.modified()?)
            .unwrap_or_default()
            < CACHE_TTL),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
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
            let packages = self.packages(&config).await?;
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

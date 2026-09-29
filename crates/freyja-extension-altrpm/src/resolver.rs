use crate::{
    error::AltRpmError,
    helpers::{cache_is_fresh, parse_packages},
    models::{AltRpmDependency, Package, Repository, RepositoryTransport, default_repositories},
};
use freyja_core::{
    error::Error,
    spec::{DependencyResolver, DependencySpec, ResolvedDependency},
};
use std::{collections::BTreeMap, io::Read, path::PathBuf};
use suppaftp::{tokio::AsyncFtpStream, types::FileType};
use tokio::io::AsyncReadExt;
use tokio::sync::Mutex;
use xz2::read::XzDecoder;

pub struct AltRpmResolver {
    repositories: BTreeMap<String, Repository>,
    cache_dir: PathBuf,
    package_lists: Mutex<BTreeMap<(String, String), BTreeMap<String, Package>>>,
}

impl AltRpmResolver {
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
        let repositories = default_repositories();

        Self {
            repositories,
            cache_dir: cache_dir.into(),
            package_lists: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn with_repository(
        mut self,
        name: impl Into<String>,
        host: impl Into<String>,
        base_path: impl Into<String>,
        transport: RepositoryTransport,
    ) -> Self {
        self.repositories.insert(
            name.into(),
            Repository {
                host: host.into(),
                base_path: base_path.into(),
                transport,
            },
        );

        self
    }

    async fn resolve_package(&self, config: &AltRpmDependency) -> Result<Package, AltRpmError> {
        let repository = self.repositories.get(&config.repository).ok_or_else(|| {
            AltRpmError::UnknownRepository {
                repository: config.repository.clone(),
            }
        })?;

        self.ensure_packages_loaded(&config.repository, repository, &config.arch)
            .await?;

        let key = (config.repository.clone(), config.arch.clone());

        let cache = self.package_lists.lock().await;

        cache
            .get(&key)
            .and_then(|packages| packages.get(&config.package))
            .cloned()
            .ok_or_else(|| AltRpmError::PackageNotFound {
                package: config.package.clone(),
                repository: config.repository.clone(),
                arch: config.arch.clone(),
            })
    }

    async fn ensure_packages_loaded(
        &self,
        repository_name: &str,
        repository: &Repository,
        arch: &str,
    ) -> Result<(), AltRpmError> {
        let key = (repository_name.to_owned(), arch.to_owned());

        {
            let cache = self.package_lists.lock().await;

            if cache.contains_key(&key) {
                return Ok(());
            }
        }

        let compressed = self
            .load_pkglist_bytes(repository_name, repository, arch)
            .await?;

        let mut decoder = XzDecoder::new_multi_decoder(compressed.as_slice());
        let mut bytes = Vec::new();

        decoder
            .read_to_end(&mut bytes)
            .map_err(|source| AltRpmError::Decompression { source })?;

        let packages = parse_packages(&bytes)?;

        self.package_lists.lock().await.insert(key, packages);

        Ok(())
    }

    async fn load_pkglist_bytes(
        &self,
        repository_name: &str,
        repository: &Repository,
        arch: &str,
    ) -> Result<Vec<u8>, AltRpmError> {
        let cache_path = self
            .cache_dir
            .join(repository_name)
            .join(arch)
            .join("pkglist.classic.xz");

        if cache_is_fresh(&cache_path).await? {
            return tokio::fs::read(&cache_path)
                .await
                .map_err(|source| AltRpmError::CacheIo {
                    path: cache_path,
                    source,
                });
        }

        let bytes = self
            .fetch_pkglist(repository_name, repository, arch)
            .await?;

        if let Some(parent) = cache_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|source| AltRpmError::CacheIo {
                    path: parent.to_path_buf(),
                    source,
                })?;
        }

        tokio::fs::write(&cache_path, &bytes)
            .await
            .map_err(|source| AltRpmError::CacheIo {
                path: cache_path,
                source,
            })?;

        Ok(bytes)
    }

    async fn fetch_pkglist(
        &self,
        _repository_name: &str,
        repository: &Repository,
        arch: &str,
    ) -> Result<Vec<u8>, AltRpmError> {
        match repository.transport {
            RepositoryTransport::Http => {
                Err(AltRpmError::UnsupportedTransport { transport: "HTTP" })
            }

            RepositoryTransport::Ftp => {
                self.fetch_pkglist_ftp(_repository_name, repository, arch)
                    .await
            }
        }
    }

    async fn fetch_pkglist_ftp(
        &self,
        _repository_name: &str,
        repository: &Repository,
        arch: &str,
    ) -> Result<Vec<u8>, AltRpmError> {
        let address = format!("{}:21", repository.host);

        let mut ftp =
            AsyncFtpStream::connect(&address)
                .await
                .map_err(|source| AltRpmError::Ftp {
                    host: repository.host.clone(),
                    source: Box::new(source),
                })?;

        ftp.login("anonymous", "freyja@localhost")
            .await
            .map_err(|source| AltRpmError::Ftp {
                host: repository.host.clone(),
                source: Box::new(source),
            })?;

        ftp.transfer_type(FileType::Binary)
            .await
            .map_err(|source| AltRpmError::Ftp {
                host: repository.host.clone(),
                source: Box::new(source),
            })?;

        ftp.set_passive_nat_workaround(true);

        let path = format!(
            "{}/{}/base/pkglist.classic.xz",
            repository.base_path.trim_end_matches('/'),
            arch,
        );

        let mut stream = ftp
            .retr_as_stream(&path)
            .await
            .map_err(|source| AltRpmError::Ftp {
                host: repository.host.clone(),
                source: Box::new(source),
            })?;

        let mut bytes = Vec::new();

        stream
            .read_to_end(&mut bytes)
            .await
            .map_err(|source| AltRpmError::Ftp {
                host: repository.host.clone(),
                source: Box::new(source),
            })?;

        stream.finish().await.map_err(|source| AltRpmError::Ftp {
            host: repository.host.clone(),
            source: Box::new(source),
        })?;

        let _ = ftp.quit().await;

        Ok(bytes)
    }
}

impl Default for AltRpmResolver {
    fn default() -> Self {
        Self::new(".freyja/cache/alt-rpm/")
    }
}

#[async_trait::async_trait]
impl DependencyResolver for AltRpmResolver {
    fn kind(&self) -> &'static str {
        "alt_rpm"
    }

    async fn resolve(&self, dependency: &DependencySpec) -> Result<ResolvedDependency, Error> {
        let config: AltRpmDependency = dependency.config.clone().try_into().map_err(|source| {
            Error::InvalidDependencyConfig {
                kind: self.kind().to_owned(),
                source,
            }
        })?;

        let package =
            self.resolve_package(&config)
                .await
                .map_err(|source| Error::DependencyResolution {
                    kind: self.kind().to_owned(),
                    reference: format!("{}/{}/{}", config.repository, config.arch, config.package),
                    source: Box::new(source),
                })?;

        let reference = format!("{}/{}/{}", config.repository, config.arch, config.package);

        let fingerprint = package.nevra();

        let mut metadata = toml::Table::new();

        metadata.insert("version".to_owned(), toml::Value::String(package.version));

        metadata.insert("release".to_owned(), toml::Value::String(package.release));

        metadata.insert("arch".to_owned(), toml::Value::String(package.arch));

        if let Some(epoch) = package.epoch {
            metadata.insert("epoch".to_owned(), toml::Value::Integer(epoch.into()));
        }

        Ok(ResolvedDependency {
            kind: self.kind().to_owned(),
            reference,
            fingerprint,
            metadata,
        })
    }
}

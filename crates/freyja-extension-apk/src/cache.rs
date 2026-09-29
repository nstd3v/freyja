use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use tokio::sync::Mutex;

use crate::{
    dependency::ApkDependency,
    error::ApkError,
    index::{Package, extract_index, parse_index},
};

const CACHE_TTL: Duration = Duration::from_secs(60 * 60);
const MAX_DOWNLOAD_SIZE: usize = 8 * 1024 * 1024;
const BASE_URL: &str = "https://dl-cdn.alpinelinux.org/alpine";

pub(crate) struct IndexCache {
    cache_dir: PathBuf,
    client: reqwest::Client,
    indexes: Mutex<BTreeMap<(String, String, String), BTreeMap<String, Package>>>,
}

impl crate::ApkResolver {
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            cache: IndexCache::new(cache_dir),
        }
    }
}

impl IndexCache {
    pub(crate) fn new(cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            cache_dir: cache_dir.into(),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("valid HTTP client"),
            indexes: Mutex::new(BTreeMap::new()),
        }
    }

    pub(crate) async fn packages(
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

async fn is_fresh(path: &Path) -> Result<bool, ApkError> {
    match tokio::fs::metadata(path).await {
        Ok(metadata) => Ok(SystemTime::now()
            .duration_since(metadata.modified()?)
            .unwrap_or_default()
            < CACHE_TTL),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

use crate::{
    dependency::DebDependency,
    error::DebError,
    index::{Package, decompress, parse_packages, parse_release, verify_index},
    trust::verify_inrelease,
};
use std::collections::BTreeMap;
use std::{
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, SystemTime},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
const TTL: Duration = Duration::from_secs(3600);
pub(crate) type FetchFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Vec<u8>, DebError>> + Send + 'a>>;
pub(crate) trait Transport: Send + Sync {
    fn get<'a>(&'a self, url: &'a str, max: usize) -> FetchFuture<'a>;
}
struct Http {
    client: reqwest::Client,
}
impl Transport for Http {
    fn get<'a>(&'a self, url: &'a str, max: usize) -> FetchFuture<'a> {
        Box::pin(async move {
            let mut response = self.client.get(url).send().await?.error_for_status()?;
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await? {
                if bytes.len().saturating_add(chunk.len()) > max {
                    return Err(DebError::Invalid("download exceeds limit".into()));
                }
                bytes.extend_from_slice(&chunk);
            }
            Ok(bytes)
        })
    }
}
pub(crate) struct IndexCache {
    root: PathBuf,
    transport: Arc<dyn Transport>,
    now: Arc<dyn Fn() -> i64 + Send + Sync>,
    key_override: Option<&'static [u8]>,
}
impl IndexCache {
    pub fn new(root: PathBuf) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(40))
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                if attempt.previous().len() >= 3
                    || attempt.url().scheme() != "https"
                    || attempt.url().host_str() != Some("deb.debian.org")
                    || attempt.url().port().is_some()
                {
                    attempt.stop()
                } else {
                    attempt.follow()
                }
            }))
            .build()
            .expect("valid HTTP client");
        Self {
            root,
            transport: Arc::new(Http { client }),
            now: Arc::new(|| chrono::Utc::now().timestamp()),
            key_override: None,
        }
    }
    #[cfg(test)]
    pub fn with_transport(
        root: PathBuf,
        transport: Arc<dyn Transport>,
        now: Arc<dyn Fn() -> i64 + Send + Sync>,
        key: &'static [u8],
    ) -> Self {
        Self {
            root,
            transport,
            now,
            key_override: Some(key),
        }
    }
    pub async fn packages(
        &self,
        dep: &DebDependency,
    ) -> Result<BTreeMap<String, Package>, DebError> {
        let dir = self
            .root
            .join(&dep.suite)
            .join(&dep.component)
            .join(&dep.arch);
        let release_path = dir.join("InRelease");
        if fresh(&release_path).await? {
            let candidate = async {
                let release = tokio::fs::read(&release_path).await?;
                let entry = parse_release(
                    &verify_inrelease(&release, &dep.suite, self.key_override)?,
                    &dep.suite,
                    &dep.component,
                    &dep.arch,
                    (self.now)(),
                )?;
                let index =
                    tokio::fs::read(dir.join(format!("{}.Packages.xz", entry.hash))).await?;
                verify_index(&index, &entry)?;
                parse_packages(&decompress(&index)?, &dep.arch, &dep.package)
            }
            .await;
            if let Ok(packages) = candidate {
                return Ok(packages);
            }
        }
        let release = self
            .transport
            .get(
                &format!("{}/dists/{}/InRelease", dep.base(), dep.suite),
                4 * 1024 * 1024,
            )
            .await?;
        let entry = parse_release(
            &verify_inrelease(&release, &dep.suite, self.key_override)?,
            &dep.suite,
            &dep.component,
            &dep.arch,
            (self.now)(),
        )?;
        let index = self
            .transport
            .get(
                &format!(
                    "{}/dists/{}/{}/binary-{}/Packages.xz",
                    dep.base(),
                    dep.suite,
                    dep.component,
                    dep.arch
                ),
                64 * 1024 * 1024,
            )
            .await?;
        verify_index(&index, &entry)?;
        let packages = parse_packages(&decompress(&index)?, &dep.arch, &dep.package)?;
        // Commit verified bytes only. Index is content-addressed; InRelease is replaced last.
        tokio::fs::create_dir_all(&dir).await?;
        atomic_write(&dir.join(format!("{}.Packages.xz", entry.hash)), &index).await?;
        atomic_write(&release_path, &release).await?;
        Ok(packages)
    }
}
async fn fresh(path: &Path) -> Result<bool, DebError> {
    match tokio::fs::metadata(path).await {
        Ok(meta) => Ok(SystemTime::now()
            .duration_since(meta.modified()?)
            .is_ok_and(|age| age < TTL)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}
async fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), DebError> {
    let tmp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let result = async {
        let mut file = tokio::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&tmp)
            .await?;
        use tokio::io::AsyncWriteExt;
        file.write_all(bytes).await?;
        file.sync_all().await?;
        tokio::fs::rename(&tmp, path).await?;
        Ok::<_, DebError>(())
    }
    .await;
    if result.is_err() {
        let _ = tokio::fs::remove_file(&tmp).await;
    }
    result
}

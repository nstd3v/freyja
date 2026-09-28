use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Debug)]
pub struct HeaderEntry {
    pub tag: u32,
    pub kind: u32,
    pub offset: usize,
    pub count: u32,
}

#[derive(Debug, Deserialize)]
pub struct AltRpmDependency {
    pub repository: String,
    pub arch: String,
    pub package: String,
}

#[derive(Debug, Clone)]
pub enum RepositoryTransport {
    Http,
    Ftp,
}

#[derive(Debug, Clone)]
pub struct Repository {
    pub host: String,
    pub base_path: String,
    pub transport: RepositoryTransport,
}

pub fn default_repositories() -> BTreeMap<String, Repository> {
    BTreeMap::from([(
        "sisyphus".to_owned(),
        Repository {
            host: "ftp.altlinux.org".to_owned(),
            base_path: "/pub/distributions/ALTLinux/Sisyphus".to_owned(),
            transport: RepositoryTransport::Ftp,
        },
    )])
}

#[derive(Debug, Clone)]
pub struct Package {
    pub name: String,
    pub epoch: Option<u32>,
    pub version: String,
    pub release: String,
    pub arch: String,
}

impl Package {
    pub(crate) fn nevra(&self) -> String {
        match self.epoch {
            Some(epoch) if epoch != 0 => format!(
                "{}-{}:{}-{}.{}",
                self.name, epoch, self.version, self.release, self.arch
            ),
            _ => format!(
                "{}-{}-{}.{}",
                self.name, self.version, self.release, self.arch
            ),
        }
    }
}

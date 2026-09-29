use serde::Deserialize;

use crate::error::ApkError;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ApkDependency {
    pub(crate) release: String,
    pub(crate) repository: String,
    pub(crate) arch: String,
    pub(crate) package: String,
}

impl ApkDependency {
    pub(crate) fn validate(&self) -> Result<(), ApkError> {
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

    pub(crate) fn reference(&self) -> String {
        format!(
            "{}/{}/{}/{}",
            self.release, self.repository, self.arch, self.package
        )
    }
}

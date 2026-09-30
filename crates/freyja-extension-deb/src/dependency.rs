use crate::error::DebError;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DebDependency {
    pub suite: String,
    pub component: String,
    pub arch: String,
    pub package: String,
}
impl DebDependency {
    pub fn validate(&self) -> Result<(), DebError> {
        if !matches!(
            self.suite.as_str(),
            "bookworm" | "bookworm-updates" | "bookworm-security"
        ) {
            return Err(DebError::Invalid(format!(
                "unsupported Debian suite `{}`",
                self.suite
            )));
        }
        if self.component != "main" {
            return Err(DebError::Invalid(
                "only Debian component `main` is supported".into(),
            ));
        }
        if !matches!(self.arch.as_str(), "amd64" | "arm64") {
            return Err(DebError::Invalid("unsupported Debian architecture".into()));
        }
        let b = self.package.as_bytes();
        if b.is_empty()
            || !b[0].is_ascii_lowercase() && !b[0].is_ascii_digit()
            || !b.iter().all(|b| {
                b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'+' | b'-' | b'.')
            })
        {
            return Err(DebError::Invalid("invalid Debian package name".into()));
        }
        Ok(())
    }
    pub fn reference(&self) -> String {
        format!(
            "{}/{}/{}/{}",
            self.suite, self.component, self.arch, self.package
        )
    }
    pub fn base(&self) -> &'static str {
        if self.suite == "bookworm-security" {
            "https://deb.debian.org/debian-security"
        } else {
            "https://deb.debian.org/debian"
        }
    }
}

use async_trait::async_trait;
use std::path::PathBuf;
use tokio::process::Command;

use freyja_core::{builder::Builder, error::Error, spec::TargetSpec};

pub struct BuildkitBuilder {
    program: PathBuf,
}

impl BuildkitBuilder {
    pub fn new() -> Self {
        Self {
            program: PathBuf::from("podman"),
        }
    }

    #[cfg(test)]
    fn with_program(program: PathBuf) -> Self {
        Self { program }
    }
}

impl Default for BuildkitBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Builder for BuildkitBuilder {
    async fn build(&self, name: &str, target: &TargetSpec) -> Result<(), Error> {
        let status = Command::new(&self.program)
            .arg("buildx")
            .arg("build")
            .arg("-f")
            .arg(target.build.context.join(&target.build.dockerfile))
            .arg(&target.build.context)
            .arg("-t")
            .arg(&target.image)
            .arg("--load")
            .status()
            .await?;

        if !status.success() {
            return Err(Error::BuildFailed {
                target: name.to_owned(),
                status: status.code(),
            });
        }

        Ok(())
    }
}

#[cfg(all(test, unix))]
mod tests;

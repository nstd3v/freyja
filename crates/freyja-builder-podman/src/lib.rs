use async_trait::async_trait;
use tokio::process::Command;

use freyja_core::{builder::Builder, error::Error, spec::TargetSpec};

pub struct BuildkitBuilder;

#[async_trait]
impl Builder for BuildkitBuilder {
    async fn build(&self, name: &str, target: &TargetSpec) -> Result<(), Error> {
        let status = Command::new("podman")
            .arg("buildx")
            .arg("build")
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

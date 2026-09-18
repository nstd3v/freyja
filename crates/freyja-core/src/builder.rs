use async_trait::async_trait;

use crate::{error::Error, planner::TargetPlan, spec::TargetSpec};

#[async_trait]
pub trait Builder: Send + Sync {
    async fn build(&self, name: &str, target: &TargetSpec) -> Result<(), Error>;
}

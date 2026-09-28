use async_trait::async_trait;

use crate::{error::Error, spec::TargetSpec};

#[async_trait]
pub trait Builder: Send + Sync {
    async fn build(&self, name: &str, target: &TargetSpec) -> Result<(), Error>;
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        sync::{Arc, Mutex},
    };

    use crate::spec::{Arches, BuildSpec};

    use super::*;

    #[derive(Clone, Default)]
    struct FakeBuilder {
        built: Arc<Mutex<Vec<String>>>,
        fail: bool,
    }

    impl FakeBuilder {
        fn failing() -> Self {
            Self {
                built: Arc::new(Mutex::new(Vec::new())),
                fail: true,
            }
        }

        fn built_targets(&self) -> Vec<String> {
            self.built.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl Builder for FakeBuilder {
        async fn build(&self, name: &str, _target: &TargetSpec) -> Result<(), Error> {
            self.built.lock().unwrap().push(name.to_string());

            if self.fail {
                return Err(Error::BuildFailed {
                    target: name.to_string(),
                    status: Some(1),
                });
            }

            Ok(())
        }
    }

    #[tokio::test]
    async fn fake_builder_records_target() {
        let builder = FakeBuilder::default();
        let target = target();

        builder.build("nginx", &target).await.unwrap();

        assert_eq!(builder.built_targets(), vec!["nginx".to_string()]);
    }

    #[tokio::test]
    async fn builder_error_is_propagated() {
        let builder = FakeBuilder::failing();
        let target = target();

        let result = builder.build("nginx", &target).await;

        assert!(result.is_err());

        assert_eq!(builder.built_targets(), vec!["nginx".to_string()]);
    }

    fn target() -> TargetSpec {
        TargetSpec {
            image: "registry.example.com/example/nginx".to_string(),
            tags: vec!["latest".to_string()],
            arches: vec![Arches::Amd64],
            build: BuildSpec {
                context: ".".into(),
                dockerfile: "Dockerfile".into(),
            },
            dependencies: BTreeMap::new(),
        }
    }
}

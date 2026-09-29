use crate::error::Error;
use crate::spec::deps::DependencyResolver;
use crate::spec::deps::{DependencySpec, ResolvedDependency};
use std::collections::HashMap;

pub struct ResolverRegistry {
    resolvers: HashMap<String, Box<dyn DependencyResolver>>,
}

impl ResolverRegistry {
    pub fn new() -> Self {
        Self {
            resolvers: HashMap::new(),
        }
    }

    pub fn register<R>(&mut self, resolver: R)
    where
        R: DependencyResolver + 'static,
    {
        self.resolvers
            .insert(resolver.kind().to_owned(), Box::new(resolver));
    }

    pub fn get(&self, kind: &str) -> Option<&dyn DependencyResolver> {
        self.resolvers.get(kind).map(Box::as_ref)
    }

    pub async fn resolve(&self, dependency: &DependencySpec) -> Result<ResolvedDependency, Error> {
        let resolver =
            self.resolvers
                .get(&dependency.kind)
                .ok_or_else(|| Error::UnknownDependencyType {
                    kind: dependency.kind.clone(),
                })?;

        resolver.resolve(dependency).await
    }
}

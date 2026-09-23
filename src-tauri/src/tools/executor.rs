use super::cancellation::ScopedCancellationToken;
use super::types::{ToolDefinition, ToolDomain, ToolExecutionError, ToolRequest};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, RwLock};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Standardized execution interface for a functional domain.
/// Executors implement the domain-specific business logic without making authorization decisions.
pub trait DomainExecutor: Send + Sync {
    /// Functional domain handled by this executor
    fn domain(&self) -> ToolDomain;

    /// Dispatches execution of a permitted tool request within the domain.
    fn execute<'a>(
        &'a self,
        request: &'a ToolRequest,
        definition: &'a ToolDefinition,
        cancel_token: ScopedCancellationToken,
    ) -> BoxFuture<'a, Result<serde_json::Value, ToolExecutionError>>;
}

/// Registry managing active domain executors.
#[derive(Clone, Default)]
pub struct DomainExecutorRegistry {
    executors: Arc<RwLock<HashMap<ToolDomain, Arc<dyn DomainExecutor>>>>,
}

impl DomainExecutorRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers an executor for its declared domain.
    pub fn register(&self, executor: Arc<dyn DomainExecutor>) {
        let mut lock = self
            .executors
            .write()
            .expect("DomainExecutorRegistry write lock poisoned");
        lock.insert(executor.domain(), executor);
    }

    /// Retrieves the executor responsible for a specific domain.
    pub fn get(&self, domain: &ToolDomain) -> Option<Arc<dyn DomainExecutor>> {
        let lock = self
            .executors
            .read()
            .expect("DomainExecutorRegistry read lock poisoned");
        lock.get(domain).cloned()
    }
}

// Thread pool for managing multiple JS runtime workers

use super::worker::{JsRuntimeWorker, WorkerCommand};
use crate::js::{
    config::PoolConfig,
    error::{JsPluginError, JsRuntimeError},
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};

/// Unique identifier for a loaded plugin
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct PluginId(pub String);

/// Thread pool for managing QuickJS worker threads
///
/// QuickJS contexts are `!Send`, so each worker thread owns its own runtime.
/// The pool manages multiple workers and assigns plugins to specific workers.
pub struct JsRuntimePool {
    workers: Arc<Mutex<Vec<mpsc::Sender<WorkerCommand>>>>,
    active_contexts: Arc<Mutex<HashMap<PluginId, usize>>>, // plugin_id -> worker_index
    config: PoolConfig,
}

impl JsRuntimePool {
    /// Create a new thread pool with the given configuration
    pub fn new(config: PoolConfig) -> Self {
        let mut workers = Vec::with_capacity(config.max_workers);

        // Spawn worker threads
        for i in 0..config.max_workers {
            let runtime_config = crate::js::config::RuntimeConfig::from_default(&config);
            let worker = JsRuntimeWorker::new(i, runtime_config.clone());
            let tx = worker.1;
            worker.0.run(runtime_config);
            workers.push(tx);
        }

        Self {
            workers: Arc::new(Mutex::new(workers)),
            active_contexts: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    /// Load a plugin into the pool
    ///
    /// Returns a handle that can be used to execute code in the plugin's context.
    /// Plugins are assigned to workers using round-robin.
    pub async fn load_plugin(
        &self,
        id: PluginId,
        source: String,
        filename: String,
    ) -> Result<JsRuntimeHandle, JsPluginError> {
        let worker_index = self.assign_worker(&id).await?;
        let workers = self.workers.lock().await;
        let worker_tx = workers
            .get(worker_index)
            .ok_or_else(|| JsPluginError::Runtime(JsRuntimeError::WorkerShutdown))?;

        let (tx, rx): (oneshot::Sender<Result<(), JsRuntimeError>>, _) = oneshot::channel();
        worker_tx
            .send(WorkerCommand::LoadModule {
                source,
                filename,
                reply: tx,
            })
            .await
            .map_err(|_| JsPluginError::Runtime(JsRuntimeError::WorkerShutdown))?;

        rx.await
            .map_err(|_| JsPluginError::Runtime(JsRuntimeError::WorkerShutdown))?
            .map_err(JsPluginError::Runtime)?;

        Ok(JsRuntimeHandle {
            plugin_id: id.clone(),
            worker_index,
            worker_tx: worker_tx.clone(),
        })
    }

    /// Assign a plugin to a worker using round-robin
    async fn assign_worker(&self, id: &PluginId) -> Result<usize, JsPluginError> {
        let mut contexts = self.active_contexts.lock().await;

        if let Some(&idx) = contexts.get(id) {
            return Ok(idx);
        }

        // Round-robin assignment based on current context count
        let idx = contexts.len() % self.config.max_workers;
        contexts.insert(id.clone(), idx);
        Ok(idx)
    }
}

/// Handle for executing code in a loaded plugin
///
/// This handle is cheap to clone and can be used from async contexts.
/// All calls are serialized through the worker thread's channel.
#[derive(Clone)]
pub struct JsRuntimeHandle {
    plugin_id: PluginId,
    worker_index: usize,
    worker_tx: mpsc::Sender<WorkerCommand>,
}

impl JsRuntimeHandle {
    /// Execute JavaScript code in this plugin's context
    pub async fn execute(&self, code: &str) -> Result<Value, JsRuntimeError> {
        let (tx, rx): (oneshot::Sender<Result<Value, JsRuntimeError>>, _) = oneshot::channel();
        self.worker_tx
            .send(WorkerCommand::Execute {
                code: code.to_string(),
                reply: tx,
            })
            .await
            .map_err(|_| JsRuntimeError::WorkerShutdown)?;

        rx.await.map_err(|_| JsRuntimeError::WorkerShutdown)?
    }

    /// Get the plugin ID
    pub fn plugin_id(&self) -> &PluginId {
        &self.plugin_id
    }

    /// Get the worker index this plugin is assigned to
    pub fn worker_index(&self) -> usize {
        self.worker_index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_id_can_be_created() {
        let id = PluginId("test-plugin".to_string());
        assert_eq!(id.0, "test-plugin");
    }

    #[test]
    fn plugin_id_can_be_cloned() {
        let id = PluginId("test-plugin".to_string());
        let id2 = id.clone();
        assert_eq!(id.0, id2.0);
    }

    #[test]
    fn plugin_id_can_be_hashed() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(PluginId("plugin1".to_string()));
        set.insert(PluginId("plugin2".to_string()));
        assert_eq!(set.len(), 2);
    }

    #[tokio::test]
    #[cfg(feature = "js-runtime")]
    async fn pool_creates_workers() {
        let config = PoolConfig {
            max_workers: 2,
            ..Default::default()
        };
        let pool = JsRuntimePool::new(config);
        let workers = pool.workers.lock().await;
        assert_eq!(workers.len(), 2);
    }

    #[tokio::test]
    #[cfg(feature = "js-runtime")]
    async fn handle_can_be_cloned() {
        let config = PoolConfig::default();
        let pool = JsRuntimePool::new(config);

        // Create a simple handle without loading
        let handle = JsRuntimeHandle {
            plugin_id: PluginId("test".to_string()),
            worker_index: 0,
            worker_tx: pool.workers.lock().await[0].clone(),
        };

        let handle2 = handle.clone();
        assert_eq!(handle.plugin_id().0, handle2.plugin_id().0);
    }
}

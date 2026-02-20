// JS runtime module - thread pool and worker management

pub mod pool;
pub mod worker;

// Re-export commonly used types
pub use pool::{JsRuntimeHandle, JsRuntimePool, PluginId};
pub use worker::WorkerCommand;

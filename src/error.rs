use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum SandboxError {
    #[error("module not allowed: {0}")]
    ModuleNotAllowed(String),

    #[error("execution timed out after {limit:?}")]
    ExecutionTimeout { limit: Duration },

    #[error("memory limit exceeded ({limit_bytes} bytes)")]
    MemoryLimitExceeded { limit_bytes: usize },

    #[error("JS error: {0}")]
    JsError(String),

    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

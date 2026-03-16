use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ExecutionMetrics {
    pub duration: Duration,
    pub heap_used_bytes: usize,
    pub heap_peak_bytes: usize,
    pub timed_out: bool,
    pub oom: bool,
    pub error: Option<String>,
}

impl ExecutionMetrics {
    pub fn emit(&self) {
        tracing::info!(
            target: "sandbox::metrics",
            duration_ms = self.duration.as_millis() as u64,
            heap_used_bytes = self.heap_used_bytes,
            heap_peak_bytes = self.heap_peak_bytes,
            timed_out = self.timed_out,
            oom = self.oom,
            error = self.error.as_deref().unwrap_or("none"),
            "execution complete"
        );
    }
}

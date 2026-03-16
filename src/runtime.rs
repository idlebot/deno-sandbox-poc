use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

use deno_core::JsRuntime;
use deno_core::RuntimeOptions;

use crate::error::SandboxError;
use crate::limits::{near_heap_limit_callback, HeapLimitState, LimitFlags};
use crate::metrics::ExecutionMetrics;
use crate::module_loader::AllowlistModuleLoader;
use crate::ops::result::ExecutionResult;
use crate::ops::sandbox_ext;

pub struct SandboxConfig {
    pub max_heap_mb: usize,
    pub timeout: Duration,
    pub allowed_modules: HashMap<String, String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            max_heap_mb: 64,
            timeout: Duration::from_secs(5),
            allowed_modules: HashMap::new(),
        }
    }
}

pub struct SandboxRuntime {
    config: SandboxConfig,
}

impl SandboxRuntime {
    pub fn new(config: SandboxConfig) -> Self {
        Self { config }
    }

    pub async fn execute(
        &self,
        code: &str,
        args: serde_json::Value,
    ) -> Result<(serde_json::Value, ExecutionMetrics), SandboxError> {
        let start = Instant::now();
        let flags = LimitFlags::new();
        let max_heap_bytes = self.config.max_heap_mb * 1024 * 1024;

        let args_json =
            serde_json::to_string(&args).map_err(|e| SandboxError::Internal(e.into()))?;
        let wrapper_code = format!(
            r#"import handler from "user:main";
const ctx = {{
  db: {{
    get: (collection, id) => Deno.core.ops.op_db_get(collection, id),
    put: (collection, doc) => Deno.core.ops.op_db_put(collection, doc),
    query: (collection, filter) => Deno.core.ops.op_db_query(collection, filter),
  }},
  args: {args_json},
}};
const result = await handler(ctx);
Deno.core.ops.op_set_result(JSON.stringify(result));
"#
        );

        let mut modules = self.config.allowed_modules.clone();
        modules.insert("user:main".to_string(), code.to_string());
        modules.insert("sandbox:wrapper".to_string(), wrapper_code);

        let loader = AllowlistModuleLoader::new(modules);

        // Set V8 heap limits via CreateParams
        let create_params = deno_core::v8::CreateParams::default().heap_limits(0, max_heap_bytes);

        let mut runtime = JsRuntime::new(RuntimeOptions {
            module_loader: Some(Rc::new(loader)),
            extensions: vec![sandbox_ext::init_ops_and_esm()],
            create_params: Some(create_params),
            ..Default::default()
        });

        runtime.op_state().borrow_mut().put(ExecutionResult(None));

        // Set up near-heap-limit callback with isolate handle for termination
        let heap_state = Box::new(HeapLimitState {
            oom: flags.oom.clone(),
            isolate_handle: runtime.v8_isolate().thread_safe_handle(),
        });
        let heap_state_ptr = Box::into_raw(heap_state);
        runtime.v8_isolate().add_near_heap_limit_callback(
            near_heap_limit_callback,
            heap_state_ptr as *mut std::ffi::c_void,
        );

        // Timeout watchdog
        let isolate_handle = runtime.v8_isolate().thread_safe_handle();
        let timeout = self.config.timeout;
        let timeout_flags = flags.clone();
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let cancel_clone = cancel.clone();

        let _timeout_handle = std::thread::spawn(move || {
            std::thread::sleep(timeout);
            if !cancel_clone.load(std::sync::atomic::Ordering::SeqCst) {
                timeout_flags
                    .timed_out
                    .store(true, std::sync::atomic::Ordering::SeqCst);
                isolate_handle.terminate_execution();
            }
        });

        // Load and evaluate
        let mod_id = runtime
            .load_main_es_module(&deno_core::ModuleSpecifier::parse("sandbox:wrapper").unwrap())
            .await
            .map_err(|e| self.classify_error(&flags, e.into(), max_heap_bytes))?;

        let result = runtime.mod_evaluate(mod_id);

        let event_loop_result = runtime
            .run_event_loop(deno_core::PollEventLoopOptions::default())
            .await;

        cancel.store(true, std::sync::atomic::Ordering::SeqCst);

        // Collect results before cleanup
        let event_loop_err = event_loop_result.err();
        let mod_eval_err = if event_loop_err.is_none() {
            result.await.err()
        } else {
            None
        };

        let execution_result = if event_loop_err.is_none() && mod_eval_err.is_none() {
            runtime
                .op_state()
                .borrow()
                .borrow::<ExecutionResult>()
                .0
                .clone()
        } else {
            None
        };

        let mut heap_stats = deno_core::v8::HeapStatistics::default();
        runtime.v8_isolate().get_heap_statistics(&mut heap_stats);

        // Remove callback before dropping runtime, then clean up
        runtime
            .v8_isolate()
            .remove_near_heap_limit_callback(near_heap_limit_callback, max_heap_bytes);

        let duration = start.elapsed();
        drop(runtime);
        // Safety: heap_state_ptr was allocated by Box::into_raw above and the
        // callback has been removed, so no more references exist.
        unsafe { drop(Box::from_raw(heap_state_ptr)) };

        // Process results
        if let Some(e) = event_loop_err {
            let error = self.classify_error(&flags, e.into(), max_heap_bytes);
            let metrics = ExecutionMetrics {
                duration,
                heap_used_bytes: heap_stats.used_heap_size(),
                heap_peak_bytes: heap_stats.total_heap_size(),
                timed_out: flags.is_timed_out(),
                oom: flags.is_oom(),
                error: Some(error.to_string()),
            };
            metrics.emit();
            return Err(error);
        }

        if let Some(e) = mod_eval_err {
            let error = self.classify_error(&flags, e.into(), max_heap_bytes);
            let metrics = ExecutionMetrics {
                duration,
                heap_used_bytes: heap_stats.used_heap_size(),
                heap_peak_bytes: heap_stats.total_heap_size(),
                timed_out: flags.is_timed_out(),
                oom: flags.is_oom(),
                error: Some(error.to_string()),
            };
            metrics.emit();
            return Err(error);
        }

        let return_value = match execution_result {
            Some(json_str) => {
                serde_json::from_str(&json_str).map_err(|e| SandboxError::Internal(e.into()))?
            }
            None => serde_json::Value::Null,
        };

        let metrics = ExecutionMetrics {
            duration,
            heap_used_bytes: heap_stats.used_heap_size(),
            heap_peak_bytes: heap_stats.total_heap_size(),
            timed_out: false,
            oom: false,
            error: None,
        };
        metrics.emit();

        Ok((return_value, metrics))
    }

    fn classify_error(
        &self,
        flags: &LimitFlags,
        error: anyhow::Error,
        max_heap_bytes: usize,
    ) -> SandboxError {
        if flags.is_oom() {
            SandboxError::MemoryLimitExceeded {
                limit_bytes: max_heap_bytes,
            }
        } else if flags.is_timed_out() {
            SandboxError::ExecutionTimeout {
                limit: self.config.timeout,
            }
        } else {
            SandboxError::JsError(error.to_string())
        }
    }
}

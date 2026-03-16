use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use deno_core::v8;

/// Shared state between the near-heap-limit callback and the runtime.
pub struct HeapLimitState {
    pub oom: Arc<AtomicBool>,
    pub isolate_handle: v8::IsolateHandle,
}

/// Shared flags between the timeout thread and the runtime.
#[derive(Clone, Default)]
pub struct LimitFlags {
    pub oom: Arc<AtomicBool>,
    pub timed_out: Arc<AtomicBool>,
}

impl LimitFlags {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_oom(&self) -> bool {
        self.oom.load(Ordering::SeqCst)
    }

    pub fn is_timed_out(&self) -> bool {
        self.timed_out.load(Ordering::SeqCst)
    }
}

/// V8 near-heap-limit callback. Sets the OOM flag and terminates execution.
pub extern "C" fn near_heap_limit_callback(
    data: *mut std::ffi::c_void,
    current_heap_limit: usize,
    _initial_heap_limit: usize,
) -> usize {
    let state = unsafe { &*(data as *const HeapLimitState) };
    state.oom.store(true, Ordering::SeqCst);
    state.isolate_handle.terminate_execution();
    // Give V8 more room to unwind gracefully
    current_heap_limit * 2
}

# Security Review — Sandboxed JS Executor

Findings from the initial PR review. To be addressed post-PoC.

## Critical

### `Deno.core.ops` exposed to user code

**Location**: `src/runtime_bootstrap.js`, `src/runtime.rs:51-63`

The bootstrap script references `Deno.core.ops.op_sandbox_log`, which means user code also has full access to `Deno.core.ops.*`. A user script can:

- Call `op_set_result(...)` directly, tampering with the return value
- Bypass the `ctx.db` abstraction by calling `op_db_get/put/query` directly
- Probe `Deno.core` for other internal APIs

**Fix**: Capture op references in bootstrap locals, then `delete Deno.core` before user code runs. Same for the wrapper — capture `op_set_result` and `op_db_*` before invoking the user handler.

## Medium

### `args_json` interpolation into JS source

**Location**: `src/runtime.rs:59`

`serde_json::to_string` produces valid JSON (valid JS expression), so this is safe today. But it's fragile — if refactored to accept a raw string, it becomes a code injection vector.

**Fix**: Pass args through an op (`op_get_args`) instead of string interpolation into generated JS source.

### No `globalThis` lockdown

User code can overwrite `console`, attach properties to `globalThis`, or tamper with built-in prototypes.

**Fix**: After bootstrap, `Object.freeze(globalThis.console)` and remove `Deno.core`.

## Low

### Detached timeout thread accumulates on rapid invocations

**Location**: `src/runtime.rs:102-110`

The spawned thread sleeps for the full timeout duration even after execution completes. Rapid invocations accumulate sleeping threads.

**Fix**: Use a channel or `Condvar` so the watchdog thread can wake early on cancellation.

### Heap limit can overshoot by 2x

**Location**: `src/limits.rs:43`

The near-heap-limit callback returns `current_heap_limit * 2` to give V8 room to unwind. A malicious script can consume up to 2x `max_heap_mb` before termination.

**Fix**: Document the overshoot behavior. Set V8's limit to half the actual process memory budget.

### `heap_state_ptr` leaks on panic

**Location**: `src/runtime.rs:89, 157`

If anything between `Box::into_raw` and the `unsafe { drop(Box::from_raw(...)) }` panics, the memory leaks.

**Fix**: Use a RAII scope guard instead of manual `Box::from_raw`.

### `test_memory_limit` accepts any `JsError` as valid

**Location**: `tests/integration.rs:159`

The test accepts `SandboxError::JsError(_)` as a passing case, which is overly permissive.

**Fix**: Only accept `MemoryLimitExceeded` or `ExecutionTimeout`.

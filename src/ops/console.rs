use deno_core::op2;

#[op2(fast)]
pub fn op_sandbox_log(#[string] msg: String) {
    tracing::info!(target: "sandbox::console", "{}", msg);
}

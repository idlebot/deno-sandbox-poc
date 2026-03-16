pub mod console;
pub mod db;
pub mod result;

deno_core::extension!(
    sandbox_ext,
    ops = [
        console::op_sandbox_log,
        db::op_db_get,
        db::op_db_put,
        db::op_db_query,
        result::op_set_result,
    ],
    esm_entry_point = "ext:sandbox_ext/runtime_bootstrap.js",
    esm = [dir "src", "runtime_bootstrap.js"],
);

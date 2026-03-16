use deno_core::op2;

#[op2]
#[serde]
pub fn op_db_get(#[string] collection: String, #[string] id: String) -> serde_json::Value {
    tracing::debug!(target: "sandbox::db", op = "get", collection = %collection, id = %id);
    // Stub: return a mock document
    serde_json::json!({
        "id": id,
        "collection": collection,
        "name": "Mock User",
        "email": "mock@example.com"
    })
}

#[op2]
#[string]
pub fn op_db_put(#[string] collection: String, #[serde] doc: serde_json::Value) -> String {
    tracing::debug!(target: "sandbox::db", op = "put", collection = %collection, doc = %doc);
    // Stub: return a mock ID
    "1".to_string()
}

#[op2]
#[serde]
pub fn op_db_query(
    #[string] collection: String,
    #[serde] filter: serde_json::Value,
) -> Vec<serde_json::Value> {
    tracing::debug!(target: "sandbox::db", op = "query", collection = %collection, filter = %filter);
    // Stub: return mock results
    vec![serde_json::json!({
        "id": "1",
        "collection": collection,
        "name": "Mock User",
        "email": "mock@example.com"
    })]
}

# ADR-001: Language Choice — Rust for Sandbox, Go for Platform

**Date:** 2026-03-16
**Status:** Accepted

## Context

This project is a temporal database engine built on Postgres, where user-provided JavaScript code runs as stored procedures (business logic). The system has two distinct layers:

1. **Sandbox runtime** — executes untrusted JS code with strict isolation, resource limits, and no default capabilities.
2. **Platform** — the temporal database engine, Postgres integration, API server, orchestration, and everything else.

The initial PoC was built entirely in Rust using `deno_core`. The question is whether Rust is the right choice for the full system, given that the team has little Rust experience and strong Go experience.

## Decision

**Use Rust for the sandbox runtime. Use Go for the rest of the platform. The sandbox produces declarative output — it never calls back into Go.**

The sandbox runtime remains a small, focused Rust binary. The platform layer is built in Go. Go calls Rust (which calls JS), and JS returns a declarative set of database operations that Go then executes against Postgres.

## Execution Model

The critical design constraint is **avoiding Rust→Go callbacks**. Calling from Rust into Go (via cgo or per-operation IPC) is slow, fragile, and creates tight coupling between the two layers. Instead, the sandbox is purely functional:

```
Go Platform → Rust Sandbox → JS execution → declarative operations → Go Platform → Postgres
```

1. **Go calls Rust** with the JS code, arguments, and any pre-fetched data.
2. **Rust executes JS** in the V8 sandbox. The JS function builds up a list of database operations (gets, puts, queries) as data, rather than executing them immediately.
3. **Rust returns to Go** with the declarative operation set (e.g., JSON array of `{op: "put", collection: "users", doc: {...}}`).
4. **Go executes the operations** against Postgres in a transaction, with full control over batching, ordering, and error handling.

This means the `ctx.db` API in JS collects intent rather than performing I/O:

```js
export default async function(ctx) {
  ctx.db.put("users", { name: "Alice" });       // records intent
  const users = ctx.db.query("users", {});       // reads from pre-fetched data
  return { processed: users.length };
}
```

### Why declarative output

- **No FFI/IPC on the hot path** — the boundary is crossed exactly twice (request in, response out), not per DB operation.
- **Go owns all I/O** — Postgres connections, transactions, retries, and error handling stay in Go where the team is most productive.
- **Testability** — the sandbox is a pure function: given code + input data, it returns operations. Easy to test in isolation on both sides.
- **Transactional safety** — Go can wrap the entire operation set in a single Postgres transaction, impossible if JS executed ops one at a time.

## Rationale

### Rust for the sandbox

- `deno_core` is the best available option for embedding V8 with fine-grained capability control. It starts with zero capabilities and every JS API must be explicitly granted via custom ops — exactly what we need.
- Go alternatives are significantly less mature: `v8go` has inconsistent maintenance, and `goja` (pure Go JS) lacks ES module support and the same isolation guarantees.
- Memory safety at the sandbox boundary — where untrusted code runs — is where Rust's guarantees justify the complexity cost.
- This is a well-bounded component (~500 lines of Rust) that changes infrequently once the API shape stabilizes.

### Go for the platform

- The temporal engine, Postgres layer, API server, and orchestration don't need Rust's safety guarantees. These are trusted-code paths working with well-defined interfaces.
- Go's development velocity is 3-5x higher given the team's existing experience. Async Rust patterns (`!Send`, raw pointers, `extern "C"` FFI) impose a constant learning tax that compounds across a larger codebase.
- Go's concurrency model, standard library, and ecosystem (gRPC, database drivers, observability) are well-suited to building database infrastructure.
- This matches industry precedent: Supabase runs Deno in a separate process while the rest of the platform is built in other languages.

### Alternatives considered

- **Rust calls into Go for DB ops** — Rejected. cgo has significant per-call overhead, forces Go's scheduler into awkward threading modes, and debugging across the boundary is difficult. With JS potentially calling `db.get`/`db.put` hundreds of times per invocation, this puts FFI cost on the hot path.
- **Rust owns the DB layer too** — Would keep the JS→DB path in a single process (Rust has solid Postgres drivers via `tokio-postgres`/`sqlx`), but means significantly more Rust code and expertise required for the majority of development.
- **Go embeds V8 directly** — `v8go` or similar. Avoids the cross-language boundary entirely but trades it for a less mature, less capable V8 embedding with weaker isolation controls.

## Consequences

- The sandbox is deployed as a standalone binary or sidecar, not linked into the Go platform at compile time.
- An IPC boundary (gRPC or similar) must be defined between the Go platform and the Rust sandbox. The protocol is simple: request (code + args + pre-fetched data) in, response (operations + result + metrics) out.
- The JS `ctx.db` API must be redesigned from imperative (current PoC stubs that return mock data) to declarative (collecting operations as data). Reads require a pre-fetch strategy where Go provides data upfront or the execution happens in multiple rounds.
- Rust expertise is only required for sandbox-related changes; the majority of development happens in Go.
- V8/deno_core upgrades are isolated to the sandbox crate.

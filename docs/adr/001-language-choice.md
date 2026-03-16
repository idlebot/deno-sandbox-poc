# ADR-001: Language Choice — Rust for Sandbox, Go for Platform

**Date:** 2026-03-16
**Status:** Accepted

## Context

This project is a temporal database engine built on Postgres, where user-provided JavaScript code runs as stored procedures (business logic). The system has two distinct layers:

1. **Sandbox runtime** — executes untrusted JS code with strict isolation, resource limits, and no default capabilities.
2. **Platform** — the temporal database engine, Postgres integration, API server, orchestration, and everything else.

The initial PoC was built entirely in Rust using `deno_core`. The question is whether Rust is the right choice for the full system, given that the team has little Rust experience and strong Go experience.

## Decision

**Use Rust for the sandbox runtime. Use Go for the rest of the platform.**

The sandbox runtime remains a small, focused Rust binary. The platform layer is built in Go. The two communicate via gRPC, FFI, or subprocess invocation.

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

## Consequences

- The sandbox is deployed as a standalone binary or sidecar, not linked into the Go platform at compile time.
- An IPC boundary (gRPC or similar) must be defined between the Go platform and the Rust sandbox.
- Rust expertise is only required for sandbox-related changes; the majority of development happens in Go.
- V8/deno_core upgrades are isolated to the sandbox crate.

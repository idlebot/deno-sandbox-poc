# ADR-002: Monorepo Structure and Build Tooling

**Date:** 2026-03-16
**Status:** Accepted

## Context

The system has three language ecosystems (Go, Rust, JS) plus shared schema definitions (protobuf/gRPC). We need to decide on repository structure and build tooling that supports cross-language development without excessive overhead.

The team is small, still in PoC/early development, and already absorbing Rust as a new language (see ADR-001).

## Decision

**Use a monorepo. Use each language's native toolchain plus `buf` for protobuf. Orchestrate with a top-level Makefile or Taskfile. Do not use Bazel.**

### Repository layout

```
/
  go/           # Go platform (API server, Postgres, orchestration)
  sandbox/      # Rust sandbox (deno_core V8 executor)
  js/           # Shared JS types, standard library modules for user code
  proto/        # Protobuf/gRPC schema definitions (operation format, sandbox protocol)
  Makefile      # Top-level orchestration
```

### Build tooling

- **Go:** `go build`, `go test` — standard toolchain.
- **Rust:** `cargo build`, `cargo test` — standard toolchain.
- **Protobuf/gRPC:** `buf` for linting, breaking change detection, and code generation targeting both Go and Rust.
- **Orchestration:** A top-level `Makefile` (or `Taskfile`) that wires together `cargo build`, `go build`, `buf generate`, and cross-component tests.
- **CI:** Calls the same make targets.

## Rationale

### Why monorepo

- **Atomic changes** — the Go platform and Rust sandbox are tightly coupled by the declarative operation protocol (see ADR-001). Interface changes update both sides plus the protobuf schema in a single commit/PR.
- **Simpler CI** — one pipeline builds and tests the contract between Go and Rust. No cross-repo version coordination or dependency pinning.
- **Shared schema** — protobuf definitions live in one place; `buf generate` produces Go and Rust code from the same source of truth.
- **Discoverability** — new contributors see the full system in one place.

### Why not Bazel

- **Learning curve** — Bazel is a significant investment to set up and maintain correctly, especially with Go, Rust, and protobuf rules. The team is already absorbing Rust as a new language; adding Bazel on top compounds the overhead.
- **Mature standalone toolchains** — `go build`, `cargo`, and `buf` each work well out of the box with minimal configuration. They don't need a meta-build system to coordinate them.
- **Scale mismatch** — Bazel's strengths (hermetic builds, remote caching, build graph optimization) pay off with hundreds of developers and thousands of packages. At the current team size, a Makefile provides the same coordination with a fraction of the complexity.
- **Migration path exists** — if the project reaches the scale where incremental cross-language builds matter, migrating to Bazel is feasible with a well-structured monorepo. Starting simple doesn't close that door.

### Why buf over raw protoc

- Handles linting and breaking change detection out of the box.
- Simpler configuration than managing `protoc` plugins and paths manually.
- Generates code for both Go and Rust from a single `buf.gen.yaml`.
- Widely adopted and well-maintained.

## Consequences

- All components share a single Git history and branching model.
- PRs that touch the protocol must update proto definitions, generated code, and both Go/Rust implementations together.
- The top-level Makefile must be kept in sync as new components or generation targets are added.
- Developers need Go, Rust, and buf toolchains installed locally (can be documented in a setup script or devcontainer).

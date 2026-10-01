# Builder implementation status

The active objective is the complete builder system, all applicable sample scenarios passing through real Oyzu behavior, snapshot artifacts, and CI that builds the CLI before running scenario verification. Tool installation is excluded. The checkpoints below do not redefine completion around a subset.

## Checkpoint 1: native discovery and development tasks

Implemented in Rust:

- CLI project selection, JSON discovery and grouped task listing.
- Static native discovery for Python managers, Go, Cargo, Node managers, Maven, Gradle, Ant, Docker and Helm.
- Explicit build targets, native task commands, visible unavailable integrations, same-name TOML overrides and pre_/post_ ordering.
- Unknown/ambiguous task, conflicting native manager, escaping target path and cyclic task-dependency errors.
- Real development execution through native tools; no assertion of hermeticity or managed authorization.

Verification at this checkpoint: five Rust integration tests, including discovery across 22 existing project roots; black-box CLI checks of native npm lifecycle, grouped task ambiguity, all hook failure positions, and a real Node native build/test. These exercise parts of EX-007/008/009/018, not complete end-to-end scenario acceptance. Rust formatting and lint are mandatory checks. Host-specific CI evidence is tracked through the workflow, not inferred from local success.

The GitHub Actions workflow builds/tests the CLI on Linux, Windows and macOS and uploads the compiled binaries. A dependent matrix job downloads those binaries and runs the task scenario checks. The full build-scenario matrix is still to be implemented; a green task workflow must not be reported as all 58 scenarios passing.

## Outstanding implementation requirements

| Requirement | State |
| --- | --- |
| Immutable source capture, deterministic plan, graph/platform expansion and actual build command | Pending |
| Dependency preparation and private-registry credential isolation for every native manager | Pending |
| Capability-enforced executor, cancellation/process containment and offline build actions | Pending |
| Build/test/lint/read-only formatting orchestration with native ownership and reports | Task discovery foundation only |
| Native snapshot version projection and verified dist artifacts/manifests, including failures | Pending |
| Container convenience packaging, Dockerfile contexts/materialization, multi-platform outputs | Discovery only |
| Helm dependency capture, chart output and image digest bindings | Discovery only |
| Java multi-module, Go workspace/cgo, Rust features, Node workspaces and all Python variants | Discovery only; native build integration pending |
| OCI action cache, producer evidence and snapshot publication/retry | Pending |
| Managed policy, source-control facts, service-test/sandbox negative cases and broker behavior | Pending |
| Complete scenario runner with native registry fixtures and accurate per-scenario evidence | Pending |
| Cross-host verified full builds using the compiled CLI in GitHub Actions | Task workflow introduced; full builds pending |

The implementation must not read scenario expectation JSON as instructions for manufacturing outputs. Native source/configuration determines behavior. Existing authored scenarios retain pending-implementation status until their actual acceptance cases, including negatives, are exercised. No fixture is marked passing merely because a CLI command exists.

## Next checkpoint

Implement captured source and build-plan/manifest plumbing, then the first complete Node/Python/Go source-to-snapshot build with enforced executor boundaries. Add artifact-content and failure/report assertions to the compiled-CLI scenario job, and expand the same engine across the other managers. Keep committing incremental verified changes.

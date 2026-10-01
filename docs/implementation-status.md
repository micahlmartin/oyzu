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
| Immutable source capture, deterministic plan, graph/platform expansion and actual build command | Captured source, serial plans and initial Node/npm and Go build command; graph/platform expansion pending |
| Dependency preparation and private-registry credential isolation for every native manager | Pending |
| Capability-enforced executor, cancellation/process containment and offline build actions | Docker offline executor integrated; full boundary/cancellation verification pending |
| Build/test/lint/read-only formatting orchestration with native ownership and reports | Initial Node/npm and Go orchestration; other builder integrations pending |
| Native snapshot version projection and verified dist artifacts/manifests, including failures | Initial Node packages and Go binaries; Linux CI verification added |
| Container convenience packaging, Dockerfile contexts/materialization, multi-platform outputs | Discovery only |
| Helm dependency capture, chart output and image digest bindings | Discovery only |
| Java multi-module, Go workspace/cgo, Rust features, Node workspaces and all Python variants | Discovery only; native build integration pending |
| OCI action cache, producer evidence and snapshot publication/retry | Pending |
| Managed policy, source-control facts, service-test/sandbox negative cases and broker behavior | Pending |
| Complete scenario runner with native registry fixtures and accurate per-scenario evidence | Pending |
| Cross-host verified full builds using the compiled CLI in GitHub Actions | Three-host task workflow; first Linux container build checks added |

The implementation must not read scenario expectation JSON as instructions for manufacturing outputs. Native source/configuration determines behavior. Existing authored scenarios retain pending-implementation status until their actual acceptance cases, including negatives, are exercised. No fixture is marked passing merely because a CLI command exists.

## Next checkpoint

Initial executor and report helpers are also checked in as foundations for OEP-0007 and OEP-0012. The Docker helper requires a provisioned Linux image, resolves its identity, disables container networking and mounts only supplied workspace/output directories. JUnit, Go test-event conversion and LCOV/Go coverage summaries have focused parser tests. Both helpers are now wired into the initial `oyzu build` command. Container execution, cancellation, mount containment and complete report validation still require further acceptance verification; compilation and parser tests do not establish those guarantees.

The initial Node/Go source-to-snapshot path has passed Linux CI. Next add dependency preparation and expand the same engine across Python and the other managers. Keep committing incremental verified changes.

## Checkpoint 2: captured-source primitive

Source copying now creates a separate content-identified tree, excludes common dependency/output stores, detects changed file contents during capture, rejects nonportable/colliding paths and refuses symlinks rather than following them into host files. Existing or source-nested destinations fail. Local tests prove equal content at different checkout paths shares an identity and later source edits do not change captured bytes. A Unix-only escaping-symlink test is included for CI.

This is not yet the complete source isolation contract: allowed internal links, cross-host executable-mode normalization, repository ignore rules, race-resistant handle-based filesystem access remain required. These limitations are not treated as passing the sandbox examples. The full builder catalog remains pending.

## Checkpoint 3: first captured-source build path

`oyzu build` now constructs a plan from captured source and pre-provisioned Docker image identities, then runs serial offline actions for dependency-free npm packages and simple Go applications. `oyzu build --plan` performs capture/preflight/planning without executing project commands. `oyzu inspect dist` checks recorded content digests; it does not authenticate a producer or grant release eligibility.

The plan records native commands, inferred build/test/lint/format-check tasks, hooks, target paths, source and toolchain identity, snapshot artifact names and report intents. Semantic records use RFC 8785 encoding. Build-time shell defaults follow the Linux executor, even on a Windows CLI host. Native npm lifecycle remains npm-owned. Go formatting checks do not edit the source. The Go application fixture was formatted to meet that actual check.

Bundles contain plans, execution context, native stdout/stderr, declared artifacts, JUnit and coverage reports and a success/failure manifest. Existing Oyzu bundles are retained under `.oyzu/history`; an unrelated existing `dist` is rejected. Local and CI-indicated runs remain unverified for production. The engine never pulls toolchain images during a build.

Unit tests cover location-independent plans, toolchain digest binding, hook ordering, executor shell selection, semantic encoding and detection of changed artifact bytes. A compiled-CLI Linux CI job now exercises real Node/Go artifact contents, reports, failure manifests, repeated builds, source preservation and actual container network/filesystem/environment restrictions. Its result is authoritative for those cases; broader examples are still pending.

Remaining limits include dependency acquisition, non-Node/non-Go packaging, matrices, materialization, container packaging, cache/publication, managed policy, native macOS/Windows executors, process cancellation, strict output quotas, complete report validation, authentic source-control evidence and recovery from every interrupted/faulted finalization. Inferred checks that are unavailable do not constitute passed checks. npm workspaces and projects declaring dependencies fail explicitly for now. This checkpoint does not satisfy the full objective.

### Verified CI evidence

[Run 36829019392](https://github.com/micahlmartin/oyzu/actions/runs/36829019392) passed at commit `bab3b17196434d0a408c076b6f7f0a602d9ed4b2` on 2026-10-01. All three host CLI build/test/lint jobs and compiled-CLI task jobs passed. The Linux captured-source job ran the downloaded release CLI against real Node and Go source fixtures. It verified package metadata/content, execution of the delivered Go binary, identical artifact digests on repeated builds, JUnit/coverage records, unchanged source, retained prior bundles, tamper detection, failed tests, failed nonmutating formatting, build hooks and actual container network/root-write/socket/environment restrictions.

These are partial acceptance cases for EX-009/016/018/036/042. The authored scenarios retain pending-implementation status because their complete requirements, and the remaining scenario catalog, are not yet satisfied.

## Checkpoint 4: Python acquisition and build integration

The initial Python/pip profile now has a scoped fetch broker and native wheel resolution before the final plan. The resolver runs with container networking disabled and reaches approved PyPI index/blob routes through a private, per-acquisition file channel and a loopback adapter inside that container. Host-side requests validate each redirect and bind authorization to the matching source. No upstream authorization header, host listening socket, or general CONNECT proxy is supplied to the resolver. The channel is absent from subsequent build actions.

Rust tests use real synthetic HTTP servers to check route denial, redirect authorization, non-forwarding of credentials across sources and private-channel response handling. The initial default sources are public PyPI routes; managed enrollment, user registry configuration, private-source entitlement, broker cancellation hardening and the complete cross-platform acquisition security contract remain pending. The tested synthetic credential boundary does not establish all private-manager scenarios.

The build profile currently targets static PEP 621 pure-Python projects through a pre-provisioned `python:3.12-slim-bookworm` image. Native pip resolves wheels and captures package identities, edges and digests; source-owned pip requirement hashes are checked separately. Offline actions install the captured inputs, project a PEP 440 snapshot version, invoke the native build backend, run pytest/JUnit/Cobertura and package wheels/source archives. Source-archive transport metadata is normalized for repeatability. Dependency evidence is included in the bundle. The new Linux CI case is authoritative for end-to-end support; it is not inferred from compilation or broker unit tests.

uv/Poetry lock semantics, requirements-only application packaging, legacy/dynamic metadata, source distributions requiring build preparation, native extensions, dependency cache reuse and all other manager adapters are still required. Unsupported forms must not silently become a different manager or bypass approved routes.

The uv profile uses the pre-provisioned `ghcr.io/astral-sh/uv:0.12.21-python3.12-trixie-slim` image. It invokes `uv export --locked --offline` with the image interpreter, verifies exported native hashes during acquisition, records the original lock digest and constrains all captured runtime/test packages to that export. `uv build` and `uv run --offline --no-sync` execute against the prepared environment; the source lock is not rewritten for snapshot version projection. Custom sources, absent locks and unsupported workspace forms are not silently converted to pip projects. The uv CI case includes native stale-lock rejection.

Initial Python CI exposed vendored metadata inside the setuptools wheel. The adapter now selects only top-level distribution metadata and checks its name/version against the wheel filename; focused tests also reject absent, duplicate and inconsistent identities. The first uv CI attempt exposed an unavailable Bookworm image tag; the Trixie tag above was verified against the upstream registry. Full Python build verification remains pending until the corrected native CI cases pass.

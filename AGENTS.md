# Repository instructions

## Scope
This is the public Oyzu repository. It must be buildable and usable without private repository access or a platform account.

## Development model

Oyzu is a 100% AI-built project. All first-party implementation code, tests and scripts, including fixes and refactors, are AI-generated. People define requirements, guide agents, review results and approve changes; handcrafted code patches are not our contribution workflow. Dependencies and upstream code retain their actual authorship and licenses. Follow [CONTRIBUTING.md](CONTRIBUTING.md); AI generation never replaces verification or maintainer accountability.

## Architecture rules

- Organize by subsystem responsibility: configuration, discovery, builders, tasks, planning, execution, acquisition and evidence. Read the [code map](docs/code-organization.md) before choosing a home for new behavior.
- Give each behavior and invariant one owner. Keep CLI/UI transport and presentation thin; reusable operations must not depend on argument parsers or UI state. Avoid dependency cycles and reaching into another subsystem's private implementation.
- Shared orchestration consumes subsystem contracts, not concrete adapters. Register implementations at composition points. Pass only the context an operation needs; do not introduce a universal application context or service locator to bypass boundaries.
- Use small traits at actual extension or substitution boundaries, such as builders, detectors and native managers. Prefer concrete functions/types elsewhere and enums for closed choices. Do not create speculative plugin APIs, generic frameworks or a trait for every struct.
- Keep required contracts cohesive. Represent optional capabilities explicitly; do not force unrelated methods onto every implementation or disguise unsupported operations as successful no-ops. Distinguish unsupported, not applicable and failed outcomes where callers need different behavior.
- Document a shared interface's input/output invariants, permitted effects and failure behavior beside its definition. Adding an implementation should normally change its owned module, registration and tests; new ecosystem-specific branches in shared orchestration require reconsidering the boundary.
- Keep modules/items private by default; expose the narrowest useful interface with `pub(super)` or `pub(crate)` where possible. Pass typed inputs/results across boundaries; keep format-specific parsing/serialization at their owning boundary.
- Return `Result` for expected input, native-tool and I/O failures; preserve causes and let the application boundary choose presentation and exit status. Keep dependency additions purposeful: reuse existing libraries where appropriate and check license, supported Rust version and target-platform compatibility.
- Keep code DRY by sharing behavior with the same semantics. Search for an existing owner before adding a parallel implementation. Do not merge similar-looking ecosystem code whose native rules differ, or add global `helpers`, `utils` or catch-all `core` modules.
- Separate observation, resolution/planning and side effects. Detectors return evidence; resolvers make deterministic choices; adapters declare intent; execution/acquisition owners perform effects. Keep process, network and filesystem capabilities explicit.
- Split growing modules by responsibility, with related tests and runtime assets beside their owner. Split crates only for demonstrated reuse, dependency isolation or independent distribution; no empty scaffolding or arbitrary file-size limits.
- For a changed boundary, document its responsibility and invariants and update the code map. Verify meaningful behavior, failure paths and applicable platforms; shared contracts need conformance coverage across implementations. Refactor touched code incrementally without unrelated rewrites.

## Working rules
- Preserve existing user changes and inspect the worktree before editing.
- Do not publish private platform details, credentials, or customer information.
- Keep the CLI and agent headless; the desktop interface is optional.
- Account for Windows, macOS, and Linux in applicable changes.
- Do not invoke or bundle a separate mise executable.
- Keep project configuration minimal; do not introduce a programming language.
- Link significant implementation work to its agreed design and acceptance criteria. Distinguish drafts from accepted requirements.
- Do not choose a license or copy upstream code before preserving applicable notices and checking licensing.
- Add verification appropriate to the change; never claim unperformed checks passed.
- Keep ecosystem behavior under `src/builders/<ecosystem>` behind the crate-private `Builder` contract. Split growing adapters by responsibility and native manager; keep orchestration, sandbox enforcement and bundle collection in shared engine modules. Avoid global helpers directories, manager switches in the engine, and speculative public plugin APIs. See docs/builder-code-organization.md.

## Current state
Draft visions and OEPs are indexed in docs/README.md. Implementation is now authorized and in progress. The Rust CLI sources are in src/ with tests in tests/. Track measured capabilities and remaining work in docs/implementation-status.md; do not mark full build scenarios passing based only on discovery or development task execution.

Run `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`, and `cargo fmt --all -- --check` for Rust changes. Run `python tooling/test-task-scenarios.py --cli <compiled-oyzu-path>` for real CLI task behavior. Tool installation is excluded from the current implementation goal; use explicitly provisioned native tools.

Run `node tooling/check-docs.mjs` for documentation structure. Do not imply this validates product behavior. Preserve agreed constraints in docs/decisions.md, distinguish proposed syntax from stable contracts, and never mark a design accepted without recorded maintainer review.

The examples/ tree contains design-contract fixtures for review, not an implemented acceptance suite. Keep project configuration minimal and expectations outside project roots. Validate their structure with `python tooling/check-examples.py` (Python 3.11+ and tooling/examples-requirements.txt). Do not implement a fake Oyzu engine to make example outcomes pass.

## Third-party compliance (applies to all subdirectories)

Read [the compliance policy](compliance/README.md) before importing upstream code,
adding/updating dependencies, editing notices or preparing a distribution.
Preserve original notices and provenance; a root license does not license every
dependency, plugin, asset or downloaded tool. Record third-party impact and run
`python tooling/compliance/check.py` plus its regression tests for affected changes.
Baseline regeneration is not approval. Do not remove notices, invent approvals,
weaken checks or add license exceptions to make CI pass. AI agents may prepare
review evidence but must never approve their own changes or submit the human
reviewer's approval. Unresolved obligations block affected imports/releases.
The policy and distribution audit remain provisional; @micahlmartin is the initial
technical reviewer, not an automatic legal signoff.

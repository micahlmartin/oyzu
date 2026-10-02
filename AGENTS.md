# Repository instructions

## Scope
This is the public Oyzu repository. It must be buildable and usable without private repository access or a platform account.

## Development model

Oyzu is a 100% AI-built project. All first-party implementation code, tests and scripts, including fixes and refactors, are AI-generated. People define requirements, guide agents, review results and approve changes; handcrafted code patches are not our contribution workflow. Dependencies and upstream code retain their actual authorship and licenses. Follow [CONTRIBUTING.md](CONTRIBUTING.md); AI generation never replaces verification or maintainer accountability.

## Architecture rules

Before implementation, identify the owning subsystem, the existing contract to reuse or extend, and the behavior that will verify the change. A local fix needs no separate design document. The [code map and engineering guide](docs/code-organization.md) supplies the ownership map, detailed interface guidance and review examples behind these eight rules.

Use the [extension decision guide](docs/code-organization.md#placing-a-new-extension) when adding a framework, manager or subsystem. When changing a shared contract, inspect all implementations and callers, update affected consumers together, and verify the shared obligations. A default trait method must represent valid common behavior; it must not conceal an unsupported capability.

1. **Organize around responsibilities.** Configuration, discovery, builders, tasks, planning, execution, acquisition and evidence each own their rules. Keep CLI/UI entry points thin and dependency direction explicit. Consumers use subsystem contracts; registration/composition points select concrete adapters. Avoid dependency cycles and access to another subsystem's private implementation.
2. **Give every shared rule one owner.** Search before adding another resolver, validator, scheduler or collector. Share behavior when callers have the same semantics and should change together; keep native ecosystem differences in their adapters. Do not create global `helpers`, `utils` or catch-all `core` modules.
3. **Use the smallest useful interface.** Use narrow traits for real extension or substitution boundaries, including builders and detectors; prefer concrete functions/types elsewhere and enums for closed choices. Keep capabilities cohesive and unsupported outcomes explicit. No trait per struct, speculative plugin framework, universal application context or service locator.
4. **Make boundaries explicit in Rust.** Keep items private by default and use `pub(super)` or `pub(crate)` where sufficient. Pass typed inputs/results and document shared contracts' invariants, effects and failure behavior beside their definitions. Return `Result` for expected failures, preserve causes, and let the application boundary choose presentation and exit status.
5. **Separate facts, decisions and effects.** Detectors return evidence; resolvers make deterministic choices; builders declare intent; execution and acquisition own effects. Pass only the required context and keep process, filesystem, network and credential capabilities explicit. Embedded runtime adapters follow these same boundaries and use documented, validated data contracts.
6. **Grow structure only when needed.** Split modules by responsibility, keeping focused tests and runtime assets near their owner. For a new subsystem, add a short module-level comment stating what it owns, its entry points, and which responsibilities belong elsewhere. Extract crates for demonstrated reuse, dependency isolation or independent distribution. Check dependency purpose, license, supported Rust version and platform compatibility. No empty scaffolding, arbitrary file-size limits or unrelated architectural rewrites.
7. **Make extensions local and verifiable.** A new implementation should normally change its owned module, registration and tests. Ecosystem-specific branches in shared orchestration are a signal to reconsider the contract. Verify shared obligations across implementations, meaningful failure paths and applicable Windows/macOS/Linux behavior; passing tests alone does not establish correct ownership.
8. **Leave a clear path for the next contributor.** Update interface comments and the code map when boundaries change, and feature documentation when behavior changes. Correct new boundary violations within the change; record unrelated debt separately. Apply the [boundary review examples](docs/code-organization.md#review-boundaries-in-practice) before finishing.

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

## Documentation is part of completion

- Every addition, behavior change, deprecation or removal must update its corresponding detailed documentation in the same PR. Read and follow [the documentation maintenance standard](docs/documentation.md). Do not call functionality complete while its documentation is missing or stale.
- Maintain current user-facing behavior in `docs/reference/`, linked from its index. Update existing pages in place; a checkpoint in `docs/implementation-status.md`, code comments, a PR description or a draft OEP alone does not satisfy this requirement.
- Cover purpose, prerequisites, supported platforms/versions, usage examples, inputs/defaults/precedence, outputs, errors/recovery, security and offline behavior where applicable, compatibility/migration, limitations and verification evidence. Scale detail to the behavior; do not invent support or pad irrelevant sections.
- Update implementation status for measured capability changes and the code map/interface documentation for changed boundaries. Keep intended, experimental, verified and deferred behavior explicit. Public docs must not reveal private platform implementation or credentials.
- Before finishing, review docs against the implementation, run `node tooling/check-docs.mjs` and `git diff --check`, and identify the changed documentation in the PR. For an internal-only change with no documentation impact, record the concrete reason in the PR; do not use that exception for observable behavior changes.

## Current implementation
Draft visions and OEPs are indexed in docs/README.md. Implementation is now authorized and in progress. The Rust CLI sources are in src/ with tests in tests/. Track measured capabilities and remaining work in docs/implementation-status.md; do not mark full build scenarios passing based only on discovery or development task execution.

Run `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`, and `cargo fmt --all -- --check` for Rust changes. Run `python tooling/test-task-scenarios.py --cli <compiled-oyzu-path>` for real CLI task behavior. The maintainer authorized full OEP-0003 tool-management implementation on 2026-10-02. Retain explicitly provisioned native tool flows until the new provider passes its corresponding qualification gates; do not equate an inspector or experiment with completed installation support.

Run `node tooling/check-docs.mjs` for documentation structure. Do not imply this validates product behavior. Preserve agreed constraints in docs/decisions.md, distinguish proposed syntax from stable contracts, and never mark a design accepted without recorded maintainer review.

The examples/ tree contains design-contract fixtures for review, not an implemented acceptance suite. Keep project configuration minimal and expectations outside project roots. Validate their structure with `python tooling/check-examples.py` (Python 3.11+ and tooling/examples-requirements.txt). Do not implement a fake Oyzu engine to make example outcomes pass.

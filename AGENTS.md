# Repository instructions

## Scope
This is the public Oyzu repository. It must be buildable and usable without private repository access or a platform account.

## Development model

Oyzu is a 100% AI-built project. All first-party implementation code, tests and scripts, including fixes and refactors, are AI-generated. People define requirements, guide agents, review results and approve changes; handcrafted code patches are not our contribution workflow. Dependencies and upstream code retain their actual authorship and licenses. Follow [CONTRIBUTING.md](CONTRIBUTING.md); AI generation never replaces verification or maintainer accountability.

## Architecture rules

Before implementation, answer three questions: **Who owns this behavior? Which existing contract should it use? What observable result will verify it?** A local fix needs no separate design document. The [code map and engineering guide](docs/code-organization.md) supplies ownership, extension decisions and detailed review examples behind these eight rules.

1. **Organize by responsibility.** Configuration, discovery, builders, tasks, planning, execution, acquisition and evidence own distinct rules. Keep entry points thin. Consumers use subsystem contracts; composition selects adapters. Avoid cycles, private implementation access and adapters importing CLI/UI or another ecosystem's implementation. Cross-target relationships use shared task/artifact contracts.
2. **Give shared behavior one owner.** Search before adding a resolver, validator, scheduler or collector. Reuse rules with the same semantics; migrate callers and remove superseded copies. Keep native ecosystem differences local. Similar code alone does not justify an abstraction, and growing lists of ecosystem flags signal a poor boundary. No global `helpers`, `utils` or catch-all `core`.
3. **Use interfaces where they earn their place.** Use narrow, cohesive traits for builders, detectors and real substitution boundaries; concrete functions/types elsewhere and enums for closed choices. Name the consumer and its needs first. Keep unsupported outcomes explicit. Internal traits are not a public plugin SDK. No trait per struct, speculative framework, universal context or service locator.
4. **Use Rust to enforce boundaries.** Default to private visibility; prefer `pub(super)` or `pub(crate)` over `pub`. Pass typed inputs/results, protect invariants and model exclusive outcomes with enums. Document contract guarantees, effects and failures beside definitions. Keep unstructured data at protocol boundaries. Expected failures return `Result` with causes preserved; entry points own presentation and exit status.
5. **Separate facts, decisions and effects.** Detectors report evidence; resolvers choose deterministically; builders declare intent; execution and acquisition own effects. Pass only required context and explicit process, filesystem, network and credential capabilities. Embedded runtime adapters follow the same rules through documented, validated contracts.
6. **Grow structure with demonstrated need.** Start with cohesive modules; keep focused tests and runtime assets near their owner. New subsystems document ownership, entry points and exclusions. Extract crates for concrete reuse, dependency isolation or distribution needs. Assess dependencies for purpose, license, Rust version and platform support. No empty scaffolding, arbitrary size limits or unrelated rewrites.
7. **Keep extensions local and verifiable.** Normally add an owned implementation, registration and tests. Ecosystem switches in orchestration signal a contract problem. Update changed contracts, implementations and consumers together; defaults must express valid common behavior. Test observable obligations through supported entry points and private algorithms with their owner. Never widen visibility just for tests. Verify native integration, failures and applicable Windows/macOS/Linux behavior.
8. **Help the next contributor.** Update interface comments, the code map and affected feature docs with the change. Keep shared rules here; nested instructions add only local constraints. Fix boundary violations introduced by the change and record unrelated debt separately. Apply the [boundary review examples](docs/code-organization.md#review-boundaries-in-practice); passing tests alone does not prove good ownership.

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

Run `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`, and `cargo fmt --all -- --check` for Rust changes. Run `python tooling/test-task-scenarios.py --cli <compiled-oyzu-path>` for real CLI task behavior. Tool installation is excluded from the current implementation goal; use explicitly provisioned native tools.

Run `node tooling/check-docs.mjs` for documentation structure. Do not imply this validates product behavior. Preserve agreed constraints in docs/decisions.md, distinguish proposed syntax from stable contracts, and never mark a design accepted without recorded maintainer review.

The examples/ tree contains design-contract fixtures for review, not an implemented acceptance suite. Keep project configuration minimal and expectations outside project roots. Validate their structure with `python tooling/check-examples.py` (Python 3.11+ and tooling/examples-requirements.txt). Do not implement a fake Oyzu engine to make example outcomes pass.

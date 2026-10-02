# Repository instructions

## Scope
This is the public Oyzu repository. It must be buildable and usable without private repository access or a platform account.

## Development model

Oyzu is a 100% AI-built project. All first-party implementation code, tests and scripts, including fixes and refactors, are AI-generated. People define requirements, guide agents, review results and approve changes; handcrafted code patches are not our contribution workflow. Dependencies and upstream code retain their actual authorship and licenses. Follow [CONTRIBUTING.md](CONTRIBUTING.md); AI generation never replaces verification or maintainer accountability.

## Implementation priority: prove the whole flow first

Getting the proposed solution working end to end is the first priority for every implementation. The primary purpose of implementing a proposal is to prove that its complete solution works through the product. Hardening comes afterward and has an explicitly bounded scope; it is not an open-ended prerequisite to demonstrating the feature.

**Default next action: advance the earliest unproved part of the user flow.** Before the first end-to-end proof, a component refinement is in scope only when an observed blocker or an explicit functional or safety requirement makes it necessary for that proof. After the proof, close the agreed functional scenario gaps. Do not choose speculative edge cases merely because they are easier to test than connecting the remaining subsystems.

**This delivery order takes precedence over exhaustive checklists or component-first sequences in older plans and proposals.** Before expanding a subsystem, connect it to the real user flow. An OEP's primary implementation milestone is evidence that its proposed solution works through the actual product, not completion of every defensive refinement it describes. Keep required functional and safety guarantees intact; classify later qualification separately rather than treating the entire proposal as one undifferentiated completion checklist.

**Implementation completion excludes hardening.** Complete the agreed functional flows and their end-to-end acceptance checks, then finish the implementation goal. Manual testing and maintainer evaluation determine a later hardening phase. Do not keep an implementation goal active for additional defensive checks, reproducibility refinements, stress testing or other hardening unless the user explicitly includes them. Missing functional behavior and observed failures of required user flows still block completion. Preserve existing safeguards and report known limitations; implementation-complete does not mean production-ready.

1. **Define the proof before implementing.** Identify the real user entry point, representative input, expected final result and command that demonstrates it. For significant work, put this in the OEP with linked examples. State what is needed for this first proof and what is later hardening; routine fixes need no new proposal.
2. **Connect the whole path first.** Implement the smallest real flow through the required subsystems to the usable result. For a builder, invoke the compiled CLI on a real project and produce its expected artifact and required reports. Isolated helpers, mocked success, discovery output and unit-test counts do not prove the solution.
3. **Close functional gaps before polishing edges.** After the first demonstration, prioritize the remaining required end-to-end scenarios. Do not repeatedly harden one working component while required flows remain unproved. Before taking a follow-up, state which missing flow or observed blocker it closes; otherwise record it for later hardening.
4. **Bound hardening separately.** Track additional edge cases, broader compatibility, performance tuning, resilience and defensive refinements as named follow-ups with concrete acceptance criteria. Do not silently expand the current milestone when new concerns are discovered. Fix issues that prevent the demonstration from working correctly, and preserve existing safety boundaries and explicit task requirements; this priority is not permission to expose credentials, fabricate evidence or bypass policy.
5. **Report progress against outcomes.** Distinguish end-to-end demonstrated, required scenario coverage remaining and hardening deferred. Show the actual command/result and known limits. Once a milestone's agreed checks pass, move on; do not redefine its finish line through successive refinements. A demonstrated path is not a claim that all scenarios pass or the feature is production-ready.

**Follow-up decision and stopping rule:** before adding work, name the agreed user scenario that is missing or failing and the evidence that will close it. If no such scenario is affected, defer the work unless it is explicitly requested hardening. A failed check must remain visible; explain whether it blocks the functional milestone or later qualification instead of weakening or hiding it. Do not replace a completed milestone with another round of speculative checks. Hardening has degrees, and maintainers choose the next degree after reviewing the working solution.

Apply the architecture, documentation and verification rules below in this order of delivery. They support a working solution; they must not turn the first demonstration into exhaustive subsystem hardening.

## Architecture rules

Before implementation, answer three questions: **Who owns this behavior? Which existing contract should it use? What observable result will verify it?** A local fix needs no separate design document. The [code map and engineering guide](docs/code-organization.md) supplies ownership, extension decisions and detailed review examples behind these eight rules.

Scale architectural review to the change: internal implementation, changed contract, or new dependency between subsystems. Explain changed guarantees or dependency direction in the PR when applicable; routine internal fixes need no architectural write-up. See [review scope](docs/code-organization.md#review-scope).

1. **Organize by responsibility.** Configuration, discovery, builders, tasks, planning, execution, acquisition and evidence own distinct rules. Keep entry points thin. Consumers use subsystem contracts; composition selects adapters. Avoid cycles, private implementation access and adapters importing CLI/UI or another ecosystem's implementation. Cross-target relationships use shared task/artifact contracts. Ecosystem code belongs under `src/builders/<ecosystem>` behind the crate-private `Builder` contract; follow the [builder extension guide](docs/builder-code-organization.md).
2. **Give shared behavior one owner.** Search before adding a resolver, validator, scheduler or collector. Reuse rules with the same semantics; migrate callers and remove superseded copies. Keep native ecosystem differences local. Similar code alone does not justify an abstraction, and growing lists of ecosystem flags signal a poor boundary. No global `helpers`, `utils` or catch-all `core`.
3. **Use interfaces where they earn their place.** Use narrow, cohesive traits for builders, detectors and real substitution boundaries; concrete functions/types elsewhere and enums for closed choices. Name the consumer and its needs first. Keep unsupported outcomes explicit. Internal traits are not a public plugin SDK. No trait per struct, speculative framework, universal context or service locator.
4. **Use Rust to enforce boundaries.** Default to private visibility; prefer `pub(super)` or `pub(crate)` over `pub`. Pass typed inputs/results, protect invariants and model exclusive outcomes with enums. Document contract guarantees, effects and failures beside definitions. Keep unstructured data at protocol boundaries. Expected failures return `Result` with causes preserved; entry points own presentation and exit status.
5. **Separate facts, decisions and effects.** Detectors report evidence; resolvers choose deterministically; builders declare intent; execution and acquisition own effects. Pass only required context and explicit process, filesystem, network and credential capabilities. Register settings through the configuration contract and consume resolved target snapshots; never recreate precedence or bypass constraints through direct file/environment reads. Execution consumes frozen plan values, and profiles never grant authority. Follow the [configuration reference](docs/reference/configuration.md). Embedded runtime adapters follow the same rules through documented, validated contracts.
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

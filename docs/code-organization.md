# Code organization and extension guide

This is repository-wide engineering guidance for contributors and coding agents. The concise rules live in [AGENTS.md](../AGENTS.md); the [contribution workflow](../CONTRIBUTING.md) explains how to apply them. Product direction and proposed future components live in [architecture](architecture.md). This map is not a claim that every intended subsystem or invariant is implemented.

## Find the owner first

Oyzu currently uses one Rust package with a library and CLI binary. Modules establish boundaries now; separate crates are a later option when reuse, dependency isolation or distribution warrants them. A new ecosystem does not automatically need a new crate.

| Responsibility | Current entry points | Boundary to preserve as the code grows |
| --- | --- | --- |
| CLI input and output | `src/main.rs` | Parse arguments, call operations, render results; keep business rules in the library |
| Configuration | `src/config.rs` | Own parsing, validation and effective settings; implement future precedence/provenance here rather than in each consumer |
| Discovery and resolution | `src/discovery.rs`, `src/discovery/` | Gather bounded evidence and resolve ownership/capabilities deterministically; never execute project code during static detection |
| Ecosystem integration | `src/builders/<ecosystem>/` | Own native manager/framework semantics; implement shared contracts and declare commands, artifacts and reports |
| Development tasks | `src/tasks.rs`, `src/launch.rs` | Own task lookup, ordering, hooks and host launch behavior; keep ecosystem inference in builders and distinguish development execution from isolated builds |
| Build lifecycle and planning | `src/build/` | Compose declared work, dependencies and hooks into a plan; coordinate execution and finalization without importing concrete framework implementations |
| Execution | `src/executor.rs`, `src/executor/` | Own sandbox capabilities, process/worker lifecycle, cancellation and cleanup; consume typed execution modes |
| Acquisition and transport | `src/dependencies.rs`, `src/dependencies/`, `src/broker.rs`, `src/broker/` | Own prepared input lifecycle and source/credential boundaries; native lock interpretation remains in the ecosystem adapter |
| Source and input identity | `src/snapshot.rs`, `src/snapshot/` | Own capture, projection, containment and content identity |
| Reports and artifacts | `src/reports.rs`, `src/reports/`, `src/oci/`, `src/build/collection.rs`, `src/build/bundle.rs` | Parse/verify formats separately from collection; record actual outputs and failures rather than trusting an adapter's success claim |
| Shared data contracts | `src/model.rs`, `src/records.rs`, `docs/contracts/` | Hold genuinely shared concepts and record encoding; keep subsystem-specific types with their owner |

Tool installation, environment activation, caching, agent/connectors, publishing and desktop/platform surfaces need the same ownership discipline as they arrive. Their proposed boundaries are in the architecture and OEPs. Do not create placeholder crates or put their future behavior into a general-purpose service object now.

Existing broad public modules, dynamic records and partially combined responsibilities are migration work, not a pattern to copy blindly. Improve the relevant boundary with the feature being changed; preserve observable behavior and avoid unrelated repository-wide rewrites.

## Interfaces that earn their place

### Dependency direction

Entry points call application operations. Operations coordinate subsystem contracts; concrete adapters implement those contracts and are selected at composition/registration points. Contract definitions must not depend on their implementations. Shared scheduling, task hooks and report collection must not import ecosystem-specific behavior. An ecosystem may reuse another subsystem's public internal API, but never its private implementation.

For example, a Python framework detector returns evidence through the discovery contract; the resolver selects the framework, and the Python builder declares test commands and report intent. The task engine owns hooks and ordering, execution owns process capabilities, and collection owns retaining and validating reports. Adding a Python framework should not duplicate those engine responsibilities inside Python.

Pass the smallest typed context an operation needs. Avoid a universal application context, service locator or shared mutable state that lets every subsystem reach every other one. OS-specific process, path and credential behavior belongs behind its owning boundary, with portable callers and applicable platform checks. Modules provide these boundaries today; a crate split is a separate decision.

### Choosing the interface

Use a trait when several implementations provide one capability or when a real I/O boundary needs substitution. Existing examples are `Builder`, `Detector<C>` and the Node `Manager`. The consumer-facing contract belongs beside the subsystem that defines the capability, and implementations stay with their ecosystem or backend. Registration happens at a composition boundary; it must not spread tool-name switches through the engine.

Use a concrete function for a pure transformation with one implementation. Use an enum when the engine owns a finite set of choices, such as artifact kinds or executor modes. Prefer composition to inheritance-like trait hierarchies. An internal trait is not automatically a stable public plugin API; dynamic loading and external compatibility require a separate design.

Document the contract beside the trait or entry point: what inputs must already be validated, what outputs guarantee, which effects are permitted, and how unsupported capabilities and failures are represented. State ordering or determinism requirements where they matter. Keep these comments about obligations shared by implementations; ecosystem-specific details belong with the adapter. Shared conformance checks should exercise those obligations, with native integration tests covering each adapter's behavior.

Use an extension as a practical boundary check. A new test-framework detector should generally add an owned detector, registration and focused tests; it should not teach the scheduler about that framework. A builder translates native facts into shared intent, and the engine consumes that intent through its contract. If an extension needs a genuinely new engine capability, add a typed capability at the responsible boundary and test it there instead of branching on a tool name. Concrete implementations may be named at registration/composition points; consumers depend on their contracts. This is a review heuristic, not a promise that existing interfaces never evolve.

Keep fields private when constructors enforce an invariant. Plain data records can expose fields when no invariant needs protection. Use domain structs, enums and validated identifiers rather than unstructured JSON or string flags as a new internal protocol. Serialization and native metadata formats have explicit owners; consumers receive the facts they need. Return contextual errors at boundaries, and preserve native failure evidence.

Expected configuration, process and I/O failures return `Result`; library operations do not terminate the process or panic on these inputs. Use typed error variants when callers need different recovery paths, and add context without losing the original cause. Keep recoverable failure handling separate from assertions about internal invariants. This does not require a custom error hierarchy for every module.

Before adding a dependency, check whether an existing library already owns the capability. Explain a new dependency's purpose and consider its license, supported Rust version, enabled features and Windows/macOS/Linux support. Prefer established libraries for complex standards and protocols when they fit; reuse does not require copying upstream implementation or exposing third-party types throughout the domain model.

Reusable operations take their necessary inputs explicitly. Avoid hidden dependence on process-global environment, current directory, clock or credentials. Pure resolution/planning consumes captured facts; effectful orchestration obtains those facts through declared capabilities. Builders must not create their own shortcut around acquisition, sandboxing or bundle collection.

The internal builder development-task hook may resolve a typed command (arguments and required environment) after an explicit `oyzu run` request. Static discovery never calls it. Native invocation context stays with the builder; the shared task runner launches the resulting command. Captured builds use prepared facts instead. `BuilderPlan.fixed_env` declares captured toolchain facts and input locations that effective task environments must preserve; the shared planner checks these after applying overrides.

## Reuse without coupling unrelated behavior

Before extracting shared code, ask whether its callers have the same semantics and should change together. Shared capture/cleanup lifecycle belongs in acquisition infrastructure. npm, pnpm and Yarn lock interpretation belongs to their respective managers, even when portions look similar. Small local duplication is preferable to a shared function with a growing list of ecosystem flags.

When behavior is genuinely shared, move it to a named owner with a narrow contract and migrate the relevant callers together. Do not create a second validator, resolver or configuration precedence implementation just to finish a feature. Keep authoritative rules in one place and test their consumers against that contract.

## A small change workflow

1. Locate the owner in this map and read its implementation, tests and relevant OEP/example. State the observable outcome before generating the change.
2. Add or extend the smallest necessary contract. For a new detector, return evidence and let the resolver decide; for a builder, declare work and let shared execution/collection enforce it. See the [builder extension guide](builder-code-organization.md) and [detector contract](proposals/OEP-0006-discovery-and-planning/detectors.md).
3. Keep implementation, ecosystem runtime assets and focused tests near their owner. Use `tests/` for public operation interactions, `tooling/` for repository validation/native conformance, and `examples/` for authored product contracts. Production behavior belongs in the library or owned runtime adapters, not in the acceptance harness.
4. Verify behavior and relevant failure modes with actual native tools when claiming native integration. Mock transport or deterministic inputs where useful; a mock command cannot prove a real package was built. Preserve distinct evidence for discovery, development tasks and sandboxed builds.
5. Update the relevant map/reference/status when behavior or a boundary changes. In the PR, identify the owning subsystem, any interface change, and checks actually run. A local fix does not require a new OEP; changes to public contracts or major boundaries follow the existing proposal process.

For a new subsystem, a short module-level responsibility/invariant comment, a narrow entry point and meaningful tests are enough to start. No per-function design documents, mandatory pattern catalog, line-count quotas or new architecture framework are required. Compiler visibility, review and focused conformance tests provide the first enforcement; add automated boundary checks when a recurring violation warrants them.

## Lessons from other projects

These sources inform our engineering choices; their contribution policies are not Oyzu's AI-authorship policy.

- [rust-analyzer architecture](https://rust-analyzer.github.io/book/contributing/architecture.html) documents a code map, explicit API boundaries and architectural invariants. Oyzu adopts that clarity about ownership and permitted dependencies, without reproducing its crate count.
- [uv's contribution guide](https://github.com/astral-sh/uv/blob/main/CONTRIBUTING.md#crate-structure) makes its crate dependency hierarchy inspectable. Oyzu likewise makes ownership and dependency direction visible in this map, starting with modules rather than adopting another project's crate layout.
- [Cargo's library architecture](https://doc.rust-lang.org/stable/nightly-rustc/src/cargo/lib.rs.html) separates command wrappers from reusable operations. Oyzu keeps presentation at its entry points so operations can serve CLI, CI and later agent/UI consumers.
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/future-proofing.html) and [rust-analyzer's style guide](https://rust-analyzer.github.io/book/contributing/style.html) inform narrow visibility, invariant-preserving types and deliberate API commitments. Traits serve actual boundaries rather than becoming a universal abstraction layer.
- [Bazel rules](https://bazel.build/extending/rules) distinguish analysis, declared actions and execution. Oyzu applies that separation to builder intent and shared execution while keeping project configuration minimal.

# Code organization and extension guide

This is repository-wide engineering guidance for contributors and coding agents. The concise rules live in [AGENTS.md](../AGENTS.md); the [contribution workflow](../CONTRIBUTING.md) explains how to apply them. Product direction and proposed future components live in [architecture](architecture.md). This map is not a claim that every intended subsystem or invariant is implemented.

## Find the owner first

Oyzu currently uses one Rust package with a library and CLI binary. Modules establish boundaries now; separate crates are a later option when reuse, dependency isolation or distribution warrants them. A new ecosystem does not automatically need a new crate.

| Responsibility | Current entry points | Boundary to preserve as the code grows |
| --- | --- | --- |
| CLI input and output | `src/main.rs`, `src/config_args.rs`, `src/presentation.rs` | Parse arguments, call operations, render results; keep business rules in the library |
| Invocation composition | `src/invocation.rs` | Capture configuration before shared non-executing target selection; compose subsystem operations without moving their rules into CLI parsing |
| Configuration | `src/config.rs`, `src/config/` | Own bounded source capture, typed settings, profiles, constraints, immutable resolution and edits; keep protected policy verification and effectful refresh separate from pure resolution |
| Discovery and resolution | `src/discovery.rs`, `src/discovery/` | Gather bounded evidence and resolve ownership/capabilities deterministically; never execute project code during static detection |
| Ecosystem integration | `src/builders/<ecosystem>/` | Own native manager/framework semantics; implement shared contracts and declare commands, artifacts and reports |
| Development tasks | `src/tasks.rs`, `src/launch.rs` | Own task lookup, ordering, hooks and host launch behavior; keep ecosystem inference in builders and distinguish development execution from isolated builds |
| Build lifecycle and planning | `src/build/` | Compose declared work, dependencies and hooks into a plan; coordinate execution and finalization without importing concrete framework implementations |
| Execution | `src/executor.rs`, `src/executor/` | Own sandbox capabilities, process/worker lifecycle, cancellation and cleanup; consume typed execution modes |
| Acquisition and transport | `src/dependencies.rs`, `src/dependencies/`, `src/broker.rs`, `src/broker/` | Own prepared input lifecycle and source/credential boundaries; native lock interpretation remains in the ecosystem adapter |
| Source and input identity | `src/snapshot.rs`, `src/snapshot/` | Own capture, projection, containment and content identity |
| Reports and artifacts | `src/reports.rs`, `src/reports/`, `src/oci/`, `src/build/collection.rs`, `src/build/bundle.rs` | Parse/verify formats separately from collection; record actual outputs and failures rather than trusting an adapter's success claim |
| Shared data contracts | `src/model.rs`, `src/records.rs`, `docs/contracts/` | Hold genuinely shared concepts and record encoding; keep subsystem-specific types with their owner |

Discovery source capture supports bounded binary evidence as well as strict UTF-8 metadata. Format-specific inspection stays with its detector; Helm archive inspection never extracts files, while private archive expansion belongs to the Helm execution adapter.

The broker's `http` module owns authorized fetches, deadlines, retries and payload budgets. Its private `failure` type defines sanitized channel outcomes; `broker.rs` owns routes and the session/channel lifecycle. Adapters consume this shared transport without adding manager-specific retries or exposing upstream credentials. Native checksum validation remains adapter-owned. HTTP-date parsing is delegated to `httpdate` rather than another local parser.

Helm's `quality` module owns implicit YAML formatter selection and development command adaptation. Its `runtime/quality.py` selects chart YAML and delegates formatting to native yamlfmt. Shared task hooks, build ordering and artifact gates retain their existing owners; Go-template formatting is not implied by YAML formatting support.

Configuration's pure resolver consumes captured sources and registered types. `config/session` captures filesystem/context facts, `config/operations` exposes parser-independent inspection/edit operations over supplied selection, and `config/agent` owns signed policy cache transitions through its runtime boundary. `config/locations` owns native roots and administrative file protection, including its private Darwin ACL adapter. Builder registration owns ecosystem setting definitions and deprecated input aliases; shared configuration does not name ecosystem-specific environment variables. CLI flags remain in `config_args`. `discovery/inventory` owns explicit and conventional target-directory selection; both discovery and configuration inspection consume it through invocation composition.

Tool installation, environment activation, caching, agent/connectors, publishing and desktop/platform surfaces need the same ownership discipline as they arrive. Their proposed boundaries are in the architecture and OEPs. Do not create placeholder crates or put their future behavior into a general-purpose service object now.

Configuration enforcement admits both captured builds and development task sequences before effects. Effective configuration validates the final task environment after task overrides and native adapter additions, so administrative environment restrictions cannot be bypassed by another input channel. Discovery derives single-target root operations from that target's final cascade, including replacements and removals. The policy agent alone reconciles administratively changed bootstrap records: online verification uses current pins while preserving sequence high-water state, and authorization rechecks time after transport and storage.

Build planning owns action dependency edges: actions that mutate one target workspace remain sequenced, explicit target dependencies wait for the producer's final action, and materialization retains its producer edges. Unrelated targets do not acquire ordering edges merely because their records are adjacent. `build/scheduling` owns bounded ready-action admission and worker batches; `build/execution` owns private target workspaces/output roots and ordered collection. A deferred report failure also fails its collection boundary before dependent actions are admitted. Plans freeze the smallest root/target jobs ceiling, and execution never rereads settings.

`build/directory` owns directory artifact inventories and integrity checks used by collection, bundle inspection and materialization. `snapshot` owns their shared bounded tree traversal/identity, with a read-only inventory operation that applies the same rules as copying. Artifact traversal never applies source exclusions. Native output selection remains a builder responsibility; directory content alone does not establish platform independence.

`builders/node/detection/outputs` observes native output profiles; `builders/node/application` owns Vite's currently supported literal output selection and artifact declaration. Its runtime stages the selected tree and rejects unsafe entry types before copying. Shared collection owns the authoritative inventory and digest, and materialization owns consumer copies. Node quality owns browser defaults and native-output exclusions; these details do not enter shared orchestration.

Existing broad public modules, dynamic records and partially combined responsibilities are migration work, not a pattern to copy blindly. Improve the relevant boundary with the feature being changed; preserve observable behavior and avoid unrelated repository-wide rewrites.

## Interfaces that earn their place

### Choose the smallest structure

| When adding or changing... | Start with... | Extract or expand when... |
| --- | --- | --- |
| A pure calculation or validation | A function and typed values in the owning module | Multiple callers share the same rule and should change together |
| A subsystem with several responsibilities | Private child modules and a narrow entry point | A responsibility needs its own invariants, tests or dependencies |
| A builder, detector or backend implementation | The existing capability contract and an owned implementation | A consumer needs a capability the contract cannot express; evolve it at its owner |
| A finite set of engine-owned states | An enum with explicit variants | A real need for independent implementations justifies a trait |
| Process, filesystem or network access | The existing effect-owning boundary with explicit inputs | A new adapter needs substitution or a different platform implementation |
| Reusable code across products | The existing library/module boundary | A second consumer or dependency/distribution constraint warrants a separate crate |

Do not put code in a shared module merely because two functions look alike. For example, configuration precedence should have one authoritative implementation, while npm and Poetry retain their own lockfile semantics. Conversely, adding a new test framework should not create another hook scheduler or report collector. Reuse the existing owners of those behaviors.

### Extracting shared behavior

DRY applies to knowledge and rules, not just repeated syntax. Before extracting shared code, identify the invariant both callers need, the subsystem responsible for enforcing it, and the differences that must remain native. Put the operation with that owner and migrate affected callers together; leaving parallel implementations preserves the original maintenance problem.

For example, two test frameworks may both emit JUnit. Their adapters own invocation and native report locations; the report subsystem owns common parsing and validation. Share the parser without making it select frameworks or launch tests. Conversely, similarly shaped npm and Poetry lock entries do not justify one universal lockfile interpreter.

Give the shared operation the smallest typed input that expresses its job. A pure function often suffices; introduce a trait only when callers need interchangeable implementations. Keep configuration flags and callbacks out unless they represent a concrete supported variation. Verify the invariant at its owner and check affected integrations through their supported entry points, including relevant failures. Remove obsolete copies and documentation within the change.

### Dependency direction

Entry points call application operations. Operations coordinate subsystem contracts; concrete adapters implement those contracts and are selected at composition/registration points. Contract definitions must not depend on their implementations. Shared scheduling, task hooks and report collection must not import ecosystem-specific behavior. An ecosystem may reuse another subsystem's public internal API, but never its private implementation.

For example, a Python framework detector returns evidence through the discovery contract; the resolver selects the framework, and the Python builder declares test commands and report intent. The task engine owns hooks and ordering, execution owns process capabilities, and collection owns retaining and validating reports. Adding a Python framework should not duplicate those engine responsibilities inside Python.

Pass the smallest typed context an operation needs. Avoid a universal application context, service locator or shared mutable state that lets every subsystem reach every other one. OS-specific process, path and credential behavior belongs behind its owning boundary, with portable callers and applicable platform checks. Modules provide these boundaries today; a crate split is a separate decision.

### Placing a new extension

Start with the kind of behavior being added, then find its existing owner. These are placement decisions, not requirements to introduce new abstractions.

| New behavior | Where it belongs | What stays shared |
| --- | --- | --- |
| Recognizing a framework or package manager | An ecosystem-owned detector implementing the discovery contract | Evidence ranking, ambiguity handling and selection |
| Supporting another native package manager | An owned adapter behind the ecosystem's manager contract, where one is needed | Input capture, transport, sandboxing and task scheduling |
| Adding an ecosystem-specific test command | The builder's task/report declarations and native reporting adapter | Hooks, execution, report validation and bundle collection |
| Adding a configuration setting | The owning subsystem's registered setting definition | Parsing, precedence and effective-configuration validation |
| Adding a platform service or backend | Its responsibility-specific module and the smallest consumer-facing contract | Existing identity, configuration and transport capabilities where their semantics fit |

For a shared contract change, search for all implementations and consumers before editing. Update the producer, consumers, contract comments and relevant conformance checks together. Default trait methods are appropriate only for behavior valid for every inheriting implementation; distinguish unsupported capabilities from successful work. An adapter that needs a new capability should describe that need through the contract rather than expose its concrete type to the orchestrator.

For contributors, the goal is a bounded change whose owner and verification are easy to find. A new subsystem starts with a focused module and a short responsibility comment; add child modules or extract a crate as actual responsibilities and consumers demand it.

### Choosing the interface

Use a trait when several implementations provide one capability or when a real I/O boundary needs substitution. Existing examples are `Builder`, `Detector<C>` and the Node `Manager`. The consumer-facing contract belongs beside the subsystem that defines the capability, and implementations stay with their ecosystem or backend. Registration happens at a composition boundary; it must not spread tool-name switches through the engine.

Use a concrete function for a pure transformation with one implementation. Use an enum when the engine owns a finite set of choices, such as artifact kinds or executor modes. Prefer composition to inheritance-like trait hierarchies. An internal trait is not automatically a stable public plugin API; dynamic loading and external compatibility require a separate design.

Keep required contracts cohesive as capabilities grow. For example, adding artifact signing must not require every builder to implement a dummy signing method: builders declare artifact facts, and the signing owner acts on those facts and policy. Use a separate capability trait only when a consumer actually needs substitutable implementations; a typed capability/result may be enough. An unsupported operation, an operation that is not applicable and an attempted operation that failed must remain distinguishable when the caller handles them differently. Never return success merely to satisfy a broad interface. This is guidance for evolving boundaries, not a claim that signing is implemented.

Document the contract beside the trait or entry point: what inputs must already be validated, what outputs guarantee, which effects are permitted, and how unsupported capabilities and failures are represented. State ordering or determinism requirements where they matter. Keep these comments about obligations shared by implementations; ecosystem-specific details belong with the adapter. Shared conformance checks should exercise those obligations, with native integration tests covering each adapter's behavior.

Use an extension as a practical boundary check. A new test-framework detector should generally add an owned detector, registration and focused tests; it should not teach the scheduler about that framework. A builder translates native facts into shared intent, and the engine consumes that intent through its contract. If an extension needs a genuinely new engine capability, add a typed capability at the responsible boundary and test it there instead of branching on a tool name. Concrete implementations may be named at registration/composition points; consumers depend on their contracts. This is a review heuristic, not a promise that existing interfaces never evolve.

Keep fields private when constructors enforce an invariant. Plain data records can expose fields when no invariant needs protection. Use domain structs, enums and validated identifiers rather than unstructured JSON or string flags as a new internal protocol. Serialization and native metadata formats have explicit owners; consumers receive the facts they need. Return contextual errors at boundaries, and preserve native failure evidence.

Expected configuration, process and I/O failures return `Result`; library operations do not terminate the process or panic on these inputs. Use typed error variants when callers need different recovery paths, and add context without losing the original cause. Keep recoverable failure handling separate from assertions about internal invariants. This does not require a custom error hierarchy for every module.

Before adding a dependency, check whether an existing library already owns the capability. Explain a new dependency's purpose and consider its license, supported Rust version, enabled features and Windows/macOS/Linux support. Prefer established libraries for complex standards and protocols when they fit; reuse does not require copying upstream implementation or exposing third-party types throughout the domain model.

Reusable operations take their necessary inputs explicitly. Avoid hidden dependence on process-global environment, current directory, clock or credentials. Pure resolution/planning consumes captured facts; effectful orchestration obtains those facts through declared capabilities. Builders must not create their own shortcut around acquisition, sandboxing or bundle collection.

Java managers share `builders/java/quality` for native lint/format defaults and its owned Java runtime adapter; native Maven/Gradle/Ant lifecycles remain in their manager modules. Task scheduling and artifact gates stay in the shared engine.

The pnpm and Yarn managers own registry admission and routes. `managers/registry` assembles their shared immutable-archive snapshot records; `runtime/registry-archives.mjs` captures and verifies those archives through the broker. Native lock interpretation remains in each manager's runtime adapter. pnpm serves a local archive allowlist; Yarn materializes its native offline mirror. `manager-runtime.mjs` owns private native invocation and lifecycle cleanup, allowing asynchronous native operations while pnpm serves local archives. `integrity.mjs` owns the SHA-512 verification also used by npm. These shared contracts do not move native lock or installation rules into orchestration.

Yarn's owned adapter also verifies native resolution against the parsed captured lock in a script-disabled private install before enabling lifecycle scripts. This admits native selective version overrides without adding a shared-engine override matcher or treating a suppressed lockfile write as evidence of unchanged dependency resolution.

pnpm's `runtime/pnpm-patches.mjs` admits native patch configuration and contained source files; native pnpm owns patch selection, hash verification and application. `managers/pnpm/patches.rs` records the verified source-patch identities in the manager's dependency extension. Shared registry capture continues to inventory pristine archives and does not learn native patch semantics.

Python distribution applications extend the native package plan through `builders/python/distribution_app`. The owned runtime assembles native wheel payloads and console metadata; `runtime/application.py` owns shared archive writing, archive-source testing and packaging checks for both requirements and distribution applications. Collection and task scheduling remain engine responsibilities.

Docker image preparation owns native reference requirements and OCI conversion under `builders/docker/images` and its Go runtime. `executor/images` alone exports explicitly provisioned daemon images by immutable identity. The executor's typed `ImageInput` binds relative prepared stores and digests; the worker verifies private copies before mounting them as native OCI contexts. Project code never receives daemon access. Registry acquisition can feed this content contract later without moving Dockerfile parsing or source policy into the worker.

Docker's native metadata adapter expands arguments from explicit target-platform and epoch facts supplied by preparation. The Rust metadata contract validates those facts against execution before planning. The executor owns the fixed export epoch shared with selection; target facts do not stand in for unverified worker-platform facts.

The internal builder development-task hook may resolve a typed command (arguments and required environment) after an explicit `oyzu run` request. Static discovery never calls it. Native invocation context stays with the builder; the shared task runner launches the resulting command. Captured builds use prepared facts instead. `BuilderPlan.fixed_env` declares captured toolchain facts and input locations that effective task environments must preserve; the shared planner checks these after applying overrides.

### Boundaries across implementation languages

Embedded runtime adapters follow the same ownership rules as Rust modules. Keep a Python, JavaScript, Go or shell adapter beside the ecosystem or subsystem it serves. Native package-manager semantics belong there; configuration precedence, hook scheduling and policy decisions retain their existing owners. Moving a rule into a subprocess does not create a new owner for it.

Treat exchanged arguments, environment and structured output as an internal contract. Document required fields, path roots, permitted effects and failure behavior; validate returned data at the receiving boundary. Use the existing typed Rust representation and record encoding where applicable. Do not invent an RPC framework or duplicate a schema merely to connect two internal components. When a contract changes, update producer, consumer and a meaningful integration check together.

## Reuse without coupling unrelated behavior

Before extracting shared code, ask whether its callers have the same semantics and should change together. Shared capture/cleanup lifecycle belongs in acquisition infrastructure. npm, pnpm and Yarn lock interpretation belongs to their respective managers, even when portions look similar. Small local duplication is preferable to a shared function with a growing list of ecosystem flags.

When behavior is genuinely shared, move it to a named owner with a narrow contract and migrate the relevant callers together. Do not create a second validator, resolver or configuration precedence implementation just to finish a feature. Keep authoritative rules in one place and test their consumers against that contract.

Use the reason for change as the reuse test: would fixing this rule require the same correction in every caller? If yes, give it one owner. If two native tools merely happen to use similar code today, keep their differing semantics local. When a shared API accumulates ecosystem-specific flags, reconsider which responsibility is common before adding another flag. This decision belongs in ordinary code review; it needs no additional design form.

## A small change workflow

### Review boundaries in practice

Use these examples when reviewing generated changes. They describe ownership rules, not additional runtime features or a requirement to split every subsystem into a crate.

| Proposed change | Boundary to preserve |
| --- | --- |
| A builder interprets local overrides independently | Configuration owns precedence and validation; the builder consumes resolved settings |
| A detector launches a package manager to decide what a project contains | Static detection produces evidence from supplied inputs; permitted native execution belongs to an explicit effectful operation |
| A new framework adds its own pre/post hook runner | The framework declares its task; the task engine owns hook ordering and failure propagation |
| A scheduler branches on a particular framework or package manager | The adapter declares typed intent through the shared contract; registration selects implementations |
| Two builders independently collect and validate the same report format | Reuse the report owner; keep framework-specific command construction in each adapter |
| A shared function gains flags for unrelated ecosystem exceptions | Check whether the semantics are actually shared; keep native differences in owned adapters |

A review should be able to identify the rule's owner, the contract crossing each changed boundary and the evidence that verifies it. Correct new violations within the change. Record unrelated existing debt as a bounded follow-up with its owner; do not expand a small contribution into an architectural rewrite. This is a short review habit, not a separate approval process or mandatory design document.

### Make the change

1. Locate the owner in this map and read its implementation, tests and relevant OEP/example. State the observable outcome before generating the change.
2. Add or extend the smallest necessary contract. For a new detector, return evidence and let the resolver decide; for a builder, declare work and let shared execution/collection enforce it. See the [builder extension guide](builder-code-organization.md) and [detector contract](proposals/OEP-0006-discovery-and-planning/detectors.md).
3. Keep implementation, ecosystem runtime assets and focused tests near their owner. Use `tests/` for public operation interactions, `tooling/` for repository validation/native conformance, and `examples/` for authored product contracts. Production behavior belongs in the library or owned runtime adapters, not in the acceptance harness.
4. Verify behavior and relevant failure modes with actual native tools when claiming native integration. Mock transport or deterministic inputs where useful; a mock command cannot prove a real package was built. Preserve distinct evidence for discovery, development tasks and sandboxed builds.
5. Update the owning feature reference in the same PR for every behavior change, following the [documentation maintenance standard](documentation.md). Update the map/status when boundaries or measured capabilities change. In the PR, link the documentation and identify the owning subsystem, any interface change, and checks actually run. A local fix does not require a new OEP; changes to public contracts or major boundaries follow the existing proposal process.

For a new subsystem, a short module-level responsibility/invariant comment, a narrow entry point and meaningful tests are enough to start. No per-function design documents, mandatory pattern catalog, line-count quotas or new architecture framework are required. Compiler visibility, review and focused conformance tests provide the first enforcement; add automated boundary checks when a recurring violation warrants them.

Keep the contributor path equally small: repository-wide rules live in `AGENTS.md`, contribution steps in `CONTRIBUTING.md`, and ownership in this map. Add a nested `AGENTS.md` only when a subsystem has real additional constraints; reference shared rules instead of copying them. Put contract obligations beside the interface so implementation changes and their documentation are reviewed together.

Tests should preserve those boundaries too. Consumer tests exercise supported entry points and observable guarantees; tests of private algorithms stay with the owning module. Do not make implementation details public solely for a test harness. For a detector, check evidence, ambiguity and selection outcomes through the relevant contracts, rather than asserting its private function sequence. Shared contract checks verify common promises across implementations; native adapter checks verify ecosystem-specific behavior. A private refactor that preserves the contract should not require unrelated consumers to rewrite their tests.

Captured-build acceptance has a separate test-only composition boundary: `tooling/test-build-scenarios.py` registers/selects suites and retains invocation evidence; owned modules under `tooling/build_scenarios/` assert native results. `baselines.py` owns the original cross-ecosystem artifact/hook/isolation cases. `tooling/build-scenario-tools.sh` provisions acceptance tooling and runs native probes, while `check-build-suites.py` checks registration against CI. These files must not implement missing product behavior. See [builder acceptance checks](reference/build-verification.md).

## Lessons from other projects

These sources inform our engineering choices; their contribution policies are not Oyzu's AI-authorship policy.

- [rust-analyzer architecture](https://rust-analyzer.github.io/book/contributing/architecture.html) documents a code map, explicit API boundaries and architectural invariants. Oyzu adopts that clarity about ownership and permitted dependencies, without reproducing its crate count.
- [uv's contribution guide](https://docs.astral.sh/uv/reference/contributing/#crate-structure) makes its crate dependency hierarchy inspectable. Oyzu likewise makes ownership and dependency direction visible in this map, starting with modules rather than adopting another project's crate layout.
- [The Rust compiler's source guide](https://rustc-dev-guide.rust-lang.org/compiler-src.html) explains both the build-time benefits of crate boundaries and the cost of scattering related functionality. Oyzu should extract crates for a demonstrated boundary or dependency need while keeping related behavior discoverable together.
- [Cargo's library architecture](https://doc.rust-lang.org/stable/nightly-rustc/src/cargo/lib.rs.html) separates command wrappers from reusable operations. Oyzu keeps presentation at its entry points so operations can serve CLI, CI and later agent/UI consumers.
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/future-proofing.html) and [rust-analyzer's style guide](https://rust-analyzer.github.io/book/contributing/style.html) inform narrow visibility, invariant-preserving types and deliberate API commitments. Traits serve actual boundaries rather than becoming a universal abstraction layer.
- [Bazel rules](https://bazel.build/extending/rules) distinguish analysis, declared actions and execution. Oyzu applies that separation to builder intent and shared execution while keeping project configuration minimal.
- [Pants rule concepts](https://www.pantsbuild.org/2.30/docs/writing-plugins/the-rules-api/concepts) use typed inputs/results and route effects through the engine. Oyzu takes the explicit-contract and controlled-effects principles; adopting them does not require Pants' rule engine, implicit dependency injection or a new configuration language.

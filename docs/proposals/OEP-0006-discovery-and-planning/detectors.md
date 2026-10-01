# Detectors and discovery resolution

This companion defines the implementation direction for DEC-030. The detector architecture is an agreed product/engineering constraint; the types and resolution details below remain draft internal contracts. It complements [planning](implementation.md), [builder profiles](../OEP-0014-builders-and-examples/implementation.md) and [default testing](../OEP-0014-builders-and-examples/testing.md). It is not an external plugin ABI or a new configuration language.

## Responsibilities

Discovery is a registry of specialized detectors plus a deterministic resolver. A detector reports supported findings and their evidence. It does not select itself as the winner, mutate targets/tasks, install tools or execute a build. The resolver combines compatible findings, resolves exclusive choices, assigns native workspace ownership and preserves unresolved conflicts. Builders consume resolved facts to describe tasks, preparation, outputs and reports.

Separate detectors can recognize an ecosystem, a native workspace, a package manager, a test framework, a coverage integration, a linter or a packaging input. Each detector remains small enough to own and test its native evidence rules. Shared metadata parsing and filesystem access belong to a bounded discovery context; adding another framework should not require editing a central chain of framework names.

The orchestrator coordinates detector execution and the resolver. Resolution is a separate pure component, not whichever detector ran first or a collection of mutations to shared state. Native execution, acquisition and report generation remain outside discovery.

## Internal Rust contract

Illustrative crate-private shape:

```rust
pub(crate) trait Detector: Sync {
    fn descriptor(&self) -> DetectorDescriptor;
    fn detect(&self, context: &DetectionContext<'_>) -> Result<Detection>;
}

pub(crate) enum Detection {
    NoMatch,
    Findings(Vec<Finding>),
    Deferred(Vec<MetadataRequest>),
}
```

These are conceptual signatures, not declarations of a shipped API. Small structs or stateless function adapters can implement the trait. Use private modules and typed enums/records for facts; do not introduce a generic string-to-JSON plugin protocol, dependency-injection framework or inheritance hierarchy merely to implement detection.

The descriptor declares a stable detector ID/version, recognized subject roles, supported native versions and prerequisite fact kinds. The engine validates unique IDs and prerequisite dependencies. A finding contains:

- Subject root/scope and role, such as package manager or a particular test suite.
- Candidate identity and typed facts, such as a native workspace member set or framework configuration path.
- Evidence references: contained source paths/digests and safe structural locations, or a recorded native metadata result.
- A finite evidence category: explicit declaration, recognized native configuration, convention or fallback. No probabilistic confidence score.
- Required capabilities and any compatibility or exclusive-ownership claims expressible through the finite role contract.

Findings are observations, not arbitrary commands. The selected builder/task provider translates them into executable intent later. Detector code cannot supply unrestricted host paths, credentials or execution privileges through a finding.

## Read-only context and execution stages

Build discovery reads the captured `SourceView` and already established facts. Read and parse manifests once per content identity where possible; expose bounded typed metadata to compatible detectors. A development task-list preview may read a bounded read-only checkout view, but is labelled preliminary and is never frozen build evidence. Host-installed tools, directory enumeration order, clock, shell profiles and ambient network state do not determine framework selection.

The engine runs detectors whose declared prerequisites are satisfied against an immutable view for that stage. It collects results before advancing; detectors do not observe other detectors' partially written output. Independent detectors may execute concurrently, with results normalized into a stable order. Sorting provides reproducible output, not winner precedence.

`NoMatch` means no applicable evidence. Parse errors in recognized relevant metadata, conflicting facts and unsupported versions are diagnostics, not disguised nonmatches that allow a fallback to hide the problem. Detectors that have not matched their prerequisite scope do not parse unrelated projects or turn an irrelevant file into a global failure.

Executable metadata, such as a Gradle model, returns a typed deferred request for a registered metadata provider. Only the preparation engine can approve and execute that request in the declared sandbox/toolchain. A request cannot embed an arbitrary new host command. Its inputs and output identity are recorded before dependent detectors receive its facts. Task listing never runs repository scripts simply to decide what to display.

Static detector prerequisites form an acyclic graph. Native metadata/dependency refinement uses the existing bounded planning loop, with at most eight rounds and explicit failure on nonconvergence. Detection cannot become an unbounded rule engine. The full applicable detector registry and semantic resolver version are bound into discovery/plan identity so a new competing detector can invalidate a prior selection even when the previously selected builder is unchanged.

## Resolution rules

Resolution is scoped by root, native owner, role and, where relevant, test suite or module. There is no single global winner for a repository.

1. Establish explicit target boundaries and recognized native workspace ownership. Native modules remain attributed to their workspace owner unless an adapter can prove separate execution will not duplicate work. Overlapping incompatible ownership is an error.
2. Validate facts and explicit intent. `uses` constrains builder ownership; it does not suppress lockfile conflicts, rewrite native module ownership or prove an application entrypoint exists.
3. Combine findings for compatible roles. An ecosystem, manager, test framework, linter and image context may all be present. A Dockerfile alongside application source is a packaging finding; composition or separate target ownership must follow a registered builder rule or explicit intent, never an unconditional global preference.
4. Resolve exclusive roles using versioned role rules. A native manager declaration and matching lock corroborate one choice. Conflicting manager declarations/locks produce a diagnostic rather than choosing the strongest-looking file. Explicit task overrides select the command body but retain known framework facts and required evidence.
5. Use conventional findings only when no stronger compatible native selection establishes that role. Use a documented fallback only when no existing mechanism is identified for that role and its prerequisites are satisfied. An unknown custom test script blocks blind fallback; an unsupported existing framework does not become permission to run a different suite.
6. Resolve multiple test suites independently where native configuration establishes their identities. Jest and Vitest can coexist in different packages or named suites. Two incompatible claims over the same suite are ambiguous until a native rule or narrow override resolves them.
7. Return resolved facts plus dispositions and reasons for all relevant candidates: selected, composed, superseded, conflicting or deferred. Capability limitations stay visible; the resolver must not silently discard a required test because its reporter is unimplemented.

There is no user-configured detector priority list, numeric scoring contest, first-match rule or lexicographic tie-break. Genuine ambiguity produces the evidence and smallest missing choice. Ordinary conventional repositories still require zero Oyzu configuration.

## Example

A Node workspace contains `package.json`, a matching npm lock, Vitest configuration, ESLint configuration and a Dockerfile. Specialized detectors report those independent facts. The resolver selects the Node workspace owner and npm manager, associates Vitest with the relevant test suite, composes ESLint as a lint capability, and records the Docker packaging candidate. The builder then plans native commands and the Vitest JUnit/coverage adapter. The Dockerfile does not replace the Node owner merely because its detector ran first.

If a conflicting pnpm lock is added, discovery explains the conflict. If no test framework or test script exists, the supported Node profile selects its documented default test detector result. If an existing test script invokes an unknown harness, preserve the script and request its reporting integration instead of claiming that a default runner tested the project.

## Module ownership and migration

Shared detector types, staged orchestration and role resolution belong under `src/discovery/`. Ecosystem-specific implementations belong under `src/builders/<ecosystem>/detection/`, split by native manager or framework as they grow. Register implementations at the ecosystem boundary; central orchestration consumes the interface and must not import concrete framework detectors. Shared file/parse primitives are extracted only when they have the same semantics.

The current code registers `Builder` implementations and queries their coarse `detect` methods. Node and Python package-manager inference has moved to independent detectors and an exclusive-role resolver; task discovery consumes those results. Framework inference, other ecosystems, compatible-role composition and native ownership still need migration. Do not maintain two competing inference paths or mistake wrapping the old entire conditional chain in one detector for the completed architecture.

Migrate one ecosystem with behavior-preserving conflict/task tests, then add the default-testing requirements against the same resolved profile. The internal builder contract evolves to accept resolved facts; builders do not redetect the framework and disagree with the plan. Preparation metadata can enrich that profile only before the plan is frozen. All built-in detectors ship with the CLI initially; remotely loaded implementations require a separate versioning and isolation design.

## Verification

PLAN-09 through PLAN-12 require detector unit tests and resolver integration tests for no match, conventional match, malformed metadata, unsupported versions, deferred metadata, native ownership, multiple compatible roles/suites, genuine ambiguity and explicit overrides. Permuting registration order or concurrent completion order within the same registry must yield equal resolutions and semantic plan identity. The tests must also show that irrelevant findings cannot change another ecosystem's selected roles, while newly relevant evidence or a changed detector/resolver contract invalidates affected discovery results. A changed CLI/registry identity can conservatively invalidate plans even when selected roles remain the same.

Native conformance still verifies actual build/test outputs. A detector finding is a planning fact, not proof that a framework ran or produced JUnit/coverage. Schema fields for persisted selection evidence require versioned contract fixtures before they are advertised as a stable public interface.

# Planner, graph and scheduler implementation contract

## Engine objects and ownership

The core owns ConfigurationSnapshot, SourceSnapshot, DiscoveryResult, PreparationRequest, DependencySnapshot, BuildPlan, ExecutionEnvelope, ActionResult and BuildBundle. Builders describe work; they cannot mutate the workspace, install tools directly, authorize a destination or sign results. The broker owns acquisition; executor owns process isolation; collectors own bounded report ingestion; publication is a separate consumer of the finalized bundle.

An artifact identity is `(target, variant, name)` and is fixed before bytes exist. `primary` is allowed only when the builder identifies exactly one primary output. An action id is a stable plan-local identifier derived from target/variant/operation; it is not a cache key. An action describes command/argv, tool identity, execution and target platform, input references, environment, outputs, reports, prerequisites and enforcement requirements. Native reactors/workspaces are one ownership unit unless a proven adapter can partition without duplicate work.

## Deterministic planning algorithm

1. Read management selectors and invocation context. Capture config and source into an immutable tree, excluding output/store directories. A failed preflight still creates a failure journal where storage is available.
2. Read native manifests as data. Sort candidate roots by portable path and provider id. Explicit `uses` selects a registered builder; otherwise apply descriptor-specific evidence rules, never a probabilistic guess. Competing lockfiles or plausible app entrypoints fail with the smallest corrective setting.
3. Collapse native module/workspace ownership. Root YAML selects explicit owners; subordinate native modules remain attributable outputs, not redundant targets. Discover tasks and merge overrides/hooks with origin records.
4. Resolve requested targets and data/order dependencies. Classify each edge as ordering, runtime artifact, execution tool, generated source, report prerequisite or typed metadata binding. Reject cycles with the shortest available path diagnostic.
5. Expand finite variants; propagate runtime constraints backward only along runtime-artifact edges. Execution tools follow the action executor. Check OS/arch/ABI/runtime/base compatibility. Choose supported executors or fail before application execution.
6. Resolve tool identities and prepare dependency closure through OEP-0017. Dynamic metadata evaluation is a recorded isolated preparation action. It may add acquired inputs; it cannot secretly execute the final build. Repeat dependency closure discovery until stable, with a maximum 8 rounds and explicit failure on nonconvergence.
7. Select versions and output contracts. Add required checks from the verified policy snapshot. Bind captured external digests and symbolic producer outputs, then validate collisions, resource bounds and required executor/report capabilities.
8. Serialize the immutable semantic plan and its digest. Persist a run envelope with authorization expiries, source-provider observations and execution instance identity outside that digest. Only then schedule final build actions.

Planning can acquire metadata and run constrained preparation; `oyzu build --plan` performs complete planning but no final application actions or publication. A lightweight future discovery preview must not masquerade as this plan. Plan inspection cannot run an untrusted repository's executable metadata without repository authorization. CI never prompts: missing authorization is a typed error.

## Plan versus action identity

The plan contains symbolic references to generated outputs, not fabricated content digests. Its digest covers the versioned semantic plan. Action cache keys are computed just before dispatch after all prerequisite outputs are verified. They cover the action recipe plus actual input tree/tool/dependency digests and execution-affecting constraints. This avoids a circular requirement to know build output bytes before freezing the plan.

Use SHA-256 with domain separation and RFC 8785 canonical JSON as specified by OEP-0019. Sort semantically unordered sets (targets, edges, artifacts, input maps) before serialization; preserve argv and other ordered data. Exclude timestamps, run ids, credential handles, routing ports, incidental paths and policy revision identifiers from computation keys. Include the resolved policy requirements that change computation; the plan still records the selected revision as provenance. Equal semantic inputs under the same policy revision yield equal plan digests; token renewal alone never changes an action key.

## Scheduling and failure propagation

Kahn topological scheduling uses a stable `(target, variant, operation, action id)` ready-queue order. Actual start/completion order can vary without changing identity. Default CPU budget is available logical processors, minimum one; memory-heavy adapters request explicit reservations. A native parallel scheduler receives the action's allotted budget to avoid nested oversubscription. An action exceeding its enforced memory/time limit fails with a distinct resource reason. Default task timeout is 30 minutes, acquisition request timeout 60 seconds, overridable within policy caps and recorded in the plan.

Default builds collect failures: independent ready work may continue, dependents of failed/cancelled nodes become blocked, and required failed/blocked checks make the overall build fail. `--fail-fast` prevents starting new work after the first failure but lets safe collectors finish. Ctrl-C requests cancellation, allows 10 seconds for process-tree termination, then forces termination and finalizes an interrupted bundle. A second interrupt forces immediate exit with best-effort journal flush. Never retry a failed compiler/test automatically; adapter-declared transient acquisition retries are separate. Retries preserve attempt history.

Reserve an output destination lease at run start to avoid two concurrent invocations clobbering dist. Working runs remain in isolated engine storage. Replacing an older finalized dist bundle is an atomic directory selection under that lease, with recovery records for OS rename limitations. Preserve user-owned non-Oyzu dist contents by failing with an explicit destination conflict. `--output` selects another contained or explicitly authorized destination; it is never a build input.

## Affected builds, services and user interface

`oyzu build [target ...]` selects targets and their dependency closure. `--affected <git-ref>` compares captured source to a verified local ref and includes transitive dependents; unavailable baselines, untracked relevant files, changed builder/config/locks or unknown input scope conservatively include all affected ownership units. Report skipped targets as outside selection, not successful. Policy can expand the selection or require a full build.

Service tests create prepared service instances with scoped networks and explicit readiness/timeouts. Service startup, health, teardown and test evidence are separate records. Shared source/output directories and host sockets are denied. A networked test is explicitly classified, never counted as a network-isolated action. Failed readiness blocks the dependent test rather than fabricating test failures.

Human output explains selected targets, tool preflight, preparation, checks and dist result. `--json` emits versioned events to stdout, with diagnostics on stderr and no interleaved native output. Event categories: run-started, phase-changed, action-started, action-finished, diagnostic, bundle-finalized, run-finished. Each has run id, monotonic sequence, time and safe payload. Exit codes: 0 success; 1 action/check failure; 2 invalid config/discovery/plan; 3 authentication/policy denial; 4 missing capability/infrastructure; 5 integrity violation; 130 cancelled. Preserve contributing errors in the bundle; highest precedence is integrity, denial, invalid plan, infrastructure, action failure.

## Verification and remaining work

PLAN-01–08 cover the existing examples. Add deterministic plan snapshots, generated-input key finalization, dependency-closure nonconvergence, unknown source scopes, bounded scheduler allocation, cancellation at every phase, dist destination races and partial-failure bundles. Native Windows/macOS isolation and remote transport are capability implementations, not assumptions embedded in the planner. Initial standalone container target is proposed as linux/amd64 for a stable cross-host default; users or managed defaults can request other targets. This resolves OPEN-017 as a draft engineering choice pending review, not a newly agreed product constraint.

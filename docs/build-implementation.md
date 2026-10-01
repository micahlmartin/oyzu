# Build implementation entry point

Status: detailed **draft implementation contracts**, not accepted OEPs or completed software. The specification pass fixes initial engineering defaults, defines typed records and identifies empirical release gates. Implementers should not guess between superseded alternatives in overview text: each linked implementation companion defines the proposed v1alpha1 behavior.

## Reading and ownership map

| Concern | Implementation contract | Review fixtures |
| --- | --- | --- |
| Root, TOML/YAML, origins, overrides and bounded configuration | [OEP-0002](proposals/OEP-0002-configuration-and-management/implementation.md) | EX-001–005, EX-008, EX-032 |
| Tool preflight, frozen identities, installation transaction | [OEP-0004](proposals/OEP-0004-tool-acquisition-and-locking/implementation.md) | EX-001, EX-039, EX-047 |
| Implicit/native tasks, overrides, hooks and arguments | [OEP-0005](proposals/OEP-0005-tasks-and-hooks/implementation.md) | EX-006–010 |
| Discovery, native ownership, variants, graph, scheduler, affected builds | [OEP-0006](proposals/OEP-0006-discovery-and-planning/implementation.md) | EX-027–033, EX-044, EX-049–050 |
| Captured files, isolation, process lifecycle and executor capabilities | [OEP-0007](proposals/OEP-0007-hermetic-execution/implementation.md) | EX-042, EX-044, EX-058 |
| Local/OCI caching, identity, races, trust and retention | [OEP-0011](proposals/OEP-0011-oci-build-cache/implementation.md) | EX-034–035 |
| Journals, dist, reports, failure states and integrity | [OEP-0012](proposals/OEP-0012-build-bundles/implementation.md) | EX-036, all build examples |
| Native version mapping, evidence, signing and publication | [OEP-0013](proposals/OEP-0013-release-and-publication/implementation.md) | EX-037–038, EX-048 |
| Python/Go/Node/Rust/Java native behavior and manager profiles | [OEP-0014](proposals/OEP-0014-builders-and-examples/implementation.md) | EX-011–025, EX-051–055 |
| CI detection, central checks, policy validity and authorization | [OEP-0016](proposals/OEP-0016-platform-protocol/implementation.md) | EX-039–043, EX-048 |
| All-manager acquisition and credential boundary | [OEP-0017](proposals/OEP-0017-dependency-acquisition/README.md) | EX-051–058 |
| Optional containers, materialization, custom Dockerfiles and Helm | [OEP-0018](proposals/OEP-0018-container-and-helm-packaging/README.md) | EX-012, EX-026–030, EX-049–050, EX-056–058 |
| Record types, canonical identity and conformance layers | [OEP-0019](proposals/OEP-0019-build-records-and-conformance/README.md), [schemas](contracts/README.md) | Positive/negative schema fixtures |

Environment activation, enrollment, desktop lifecycle and general tool backends remain governed by their existing OEPs; this pass details their build-facing contracts without expanding hosted CI/deployment or choosing a license.

## Resolved draft choices

- One frozen semantic plan with symbolic producer outputs; action keys bind actual dependency outputs at dispatch. No fabricated pre-build output digests.
- Consumer-owned materialization, finite runtime/platform matrices, unique native workspace ownership and conservative affected selection.
- Whole-task override, success-only nonrecursive hooks, argv for portable commands, explicit platform-dependent shell strings.
- Frozen build inputs, broker-only upstream credentials, adapter-specific dependency stores and no network retry during execution.
- linux/amd64 as the proposed standalone container default, optional `container: true`, a native-manager selector only when custom Dockerfile dependency inference is ambiguous, and finite typed Helm image bindings.
- OCI candidate manifests with optional lookup hints; no custom cache service or assumed conditional tag writes. Strict conflict observation needs registry listing and never claims global concurrency consensus.
- Finalized immutable bundles and separate publication receipts. Cache reuse retains original producer evidence; local-origin artifacts cannot be promoted into managed production.

These choices are reviewable defaults, not additions to the agreed decision list until reviewed. Concrete proposed config variants are linked from the relevant example READMEs.

## Implementation work packages

Each row is a bounded starting issue/PR sequence, not a pre-created GitHub issue. Split large rows by module and adapter; include its OEP section, acceptance IDs, fixtures and observed checks in every implementation task.

| Order | Deliverable and dependency | Acceptance evidence required |
| --- | --- | --- |
| B01 | Contract Rust types, local schema parsing, path validation, canonical hashes; no network | RECORD-01–03; independent RFC canonicalization vectors, duplicate keys/ids, invalid refs/cycles and portable path failures |
| B02 | Config capture/origins and pure discovery with explicit native ownership; depends B01 | CFG-01/03–06, PLAN-01/02/04; zero config and root/nested/multi-module fixtures |
| B03 | Tool-lock/install store transactions and acquisition interfaces; depends B01/B02 | TOOL-01–06; locked repeat install, corrupt/traversal archives, concurrent installation, no PATH substitution |
| B04 | Task resolution, native lifecycle ownership and hook scheduler; depends B02 | TASK-01–06; three failure positions, argv edge cases, cached main with live hooks |
| B05 | Captured tree and Linux executor capability prototype; depends B01/B03 | EXEC-01/03/04/06; filesystem/network/process escape, cancellation, limits, snapshot races |
| B06 | Synthetic authenticated registry and uv acquisition/offline dependency store; depends B03/B05 | DEP-01–07 and EX-051; generated native lock, exact dependency graph, canary never visible to project code |
| B07 | Frozen plan, action scheduling, reports, failed/success dist bundle; depends B04/B05/B06 | PLAN-03/05/06, BUNDLE-01–06; one real Python build from source through verified outputs, no canned example responses |
| B08 | Go and consumer materialization, then Dockerfile-free Python image; depends B07 | PLAN-07/08, PACK-01/02/05; exact producer bytes in image, no second compile, matching ABI and no secrets |
| B09 | Remaining Python managers, npm/pnpm/Yarn, Cargo and Java adapters; depends B07 | BUILDER-01–08, DEP-01–07 per manager; native workspace/report/lock/offline/cancel matrix |
| B10 | Local action cache and OCI reference-registry integration; depends B07 | CACHE-01–06; partial upload, concurrent writers, corrupt/forged results, private-content permissions |
| B11 | Multi-platform image index, custom dependency contexts, Helm packaging/bindings; depends B08/B09 | PACK-03/04/06/07 and EX-026–030/049–058; no partial index or native-cache provenance bypass |
| B12 | Public mock policy/CI facts and required-check evaluation; depends B07 | PROTO-01–06; spoofed CI, changed scanner without repository edit, unsupported mandatory adapter, expiry/revocation |
| B13 | Version projections, bundle verification, snapshot publish/retry; depends B10/B12 | REL-03–06, RECORD-04/05; native coordinate validity, mutated bundle denial, partial receipts |
| B14 | Verified provider identity, production authorization and signing integration; depends B13 plus provider gates | REL-01/02/05/06; local cache laundering denied, wrong issuer/audience/ref denied, independently verified signatures |
| B15 | Windows/macOS host transports and lifecycle qualification; parallels B05 onward | All advertised host fixtures, no fallback to unrestricted host, real Unicode/space/path/signal behavior |

The first useful slice ends at B07; container support follows B08. This sequencing does not remove agreed Go/Node/Rust/Java/Helm support from scope. Production publishing is unavailable until B14 proves its authority boundary. No separate trusted builder code path is introduced.

## Required experiments and decisions

| Gate | Can proceed now | Evidence required before enabling the feature |
| --- | --- | --- |
| Public license/mise reuse | Independently authored types, docs and experiments | Maintainer selects license and reviews a pinned upstream import boundary/notices |
| Executor implementation | Implement capability interface and Linux prototype | Select measured backend/transport; prove confinement on Linux and virtualized Windows/macOS hosts |
| Native Windows/macOS targets | Preserve platform types and fail unsupported requests | Dedicated native executor qualification; Linux containers alone are insufficient |
| Broker endpoint/session implementation | Build synthetic-session harness and adapters | IPC/session isolation, redirect/DNS controls, expiry/rotation and canary tests on each executor |
| OCI support | Reference registry implementation | Media-type/listing/race/retention/auth compatibility per supported registry product |
| Production CI identity/signing | Typed facts, mock policies and publication state machine | Provider claims bound to captured source; independent signing and policy authorization validation |
| New config spellings | Parse/validate proposed variants | Maintainer review before documenting them as stable syntax |

No broad product question blocks implementing the pure contracts. These gates deliberately stop an unmeasured backend from being called secure or supported. They are implementation validation work, not excuses to leave plan, record or failure semantics undefined.

## Checks for this specification set

Run `node tooling/check-docs.mjs`, `python tooling/check-examples.py`, and `python tooling/check-build-contracts.py` with the documented validation dependencies. The last validates local schemas and positive/negative synthetic records; it does not validate actual cryptographic hashes, credentials, native builds or sandboxing. Future product tests must verify those properties. All OEP status remains draft/not-started until separately reviewed and implemented.

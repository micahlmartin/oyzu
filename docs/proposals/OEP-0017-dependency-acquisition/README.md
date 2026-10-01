---
id: OEP-0017
title: Package-manager acquisition and credential isolation
status: draft
implementation: not-started
updated: 2026-10-01
authors: [micahlmartin]
reviewers: []
requires: [OEP-0007, OEP-0009, OEP-0010, OEP-0014]
tracking-issue: null
---

# Package-manager acquisition and credential isolation

> Implementation specification for review. Requirements and v1alpha1 choices remain draft; no maintainer acceptance, implemented support or verified security is implied.


## Outcome and scope

Every supported package manager uses the same approved-source and credential boundary. Developers declare native dependencies and run `oyzu build`; they do not write authentication plumbing or pipeline cache steps. Connectors identify and authenticate upstream systems, package-manager adapters preserve native resolution and prepared-store semantics, and the engine enforces isolated execution. Python, npm/pnpm/Yarn, Maven/Gradle/Ant with Ivy, Go, Cargo, Helm and apt/apk each require conformance. Additional managers are added by adapters, not universal command interception.

The [EX-051–058 contract](../../../examples/DEPENDENCIES.md) supplies native fixtures and negative cases. No real credentials, public fallback requirement or private platform source is needed by the future harness. The same source runs standalone with user registry mappings and managed with centrally enforced connector routes.

## Data and interfaces

An AcquisitionRequest contains adapter id/version/digest, captured project/native-lock digests, target compatibility tuple, requested package identities/constraints, logical source ids, purpose (tool/build/runtime/test/scan), integrity requirements and allowed preparation effects. It cannot contain an arbitrary bearer URL or a command to execute outside the sandbox.

The adapter contract is `discover(native inputs) -> dependency intents`, `prepare(intents, broker session, sandbox) -> snapshot`, `offline_inputs(snapshot, execution platform) -> mount/config specification`, and `validate_complete(snapshot) -> diagnostics`. Implement as in-process versioned Rust interfaces initially, with native tools invoked through the controlled process interface. A remote plugin ABI and arbitrary project-supplied adapters are not part of the first release.

DependencySnapshot records adapter identity, manager/tool identity, native lock digests, frozen package graph, content objects, source identities, verification strength, generated build requirements, target ABI and prepared-store layout/version. Every graph node has an exact version/content identity and directed dependency edges. Missing graph details cannot be hidden behind a hash of a mutable developer cache. The snapshot digest excludes authorization tokens, route ports, cache filesystem paths and acquisition timestamps. Preserve acquisition evidence separately. The draft [snapshot schema](../../contracts/v1alpha1/dependencies.schema.json) defines the storage record.

## Preparation state machine

Discover → authorize source routes → resolve native graph → fetch/verify content → evaluate supported dynamic metadata in a sandbox → capture newly discovered requirements → freeze closure → construct credential-free prepared store → validate completeness. At most 8 closure rounds; a changing or incomplete graph fails with the responsible dependency and discovery action. Never change native versions after final plan freeze to accommodate a cache miss.

Resolution runs with approved broker egress only. The agent contacts existing approved registries/proxies directly; SaaS does not relay package bytes. A dependency's lifecycle/build/metadata script can use only an explicitly supported preparation route when required and cannot read upstream credentials. Separate package retrieval from installation scripts where native tooling permits. Where native tooling couples them, run the native resolver in an isolated acquisition sandbox with broker-scoped access and record the executable metadata effects. Unknown downloads either become explicitly supported captured inputs or fail. No generic `curl` exception.

Artifact verification happens before extraction or script execution. Capture hashes and publisher/signature evidence when available, and retain the distinction between digest integrity and authenticated provenance. Verify native lock constraints against fetched bytes. A source archive requiring build tooling records all build requirements and generated wheel/binary identities separately from upstream package bytes.

## Broker session transport

Clients request a short-lived acquisition session scoped to run, adapter, repository/source allowlist and read operations. Local host sessions use authenticated OS IPC for control. A sandbox gets a dedicated proxy endpoint or mediated fetch handle reachable only inside its authorized acquisition network. Linux virtualized/remote executors run their own acquisition session under delegated authorization; localhost inside a container is not assumed to mean the workstation.

Prefer routing that needs no credential in package-manager config. When the native protocol requires a local capability, keep that capability ephemeral, scoped to the one session, and supplied through a private temporary mount/helper. It is still a secret: do not persist it in captured caches, dependency lock URLs, logs, layers or metadata. The broker holds the upstream token in memory, renews it through the approved connector, and retains refresh credentials only in the OS secure store or CI identity exchange. Deny arbitrary proxy CONNECT, raw upstream URLs and host changes outside the authorized route.

Registry protocols can return absolute archive URLs or redirects. The adapter validates/rebinds them to approved logical routes without changing version/content identities; credential headers are never forwarded across origins by default. Enforce redirects and DNS/connection destinations at fetch time to prevent allowlist bypass. A legitimate approved upstream change requires route authorization, not blind host rewriting. If a locked URL cannot be served compatibly by the selected connector, explain the mismatch and fail.

## Private caches and offline execution

Store only package content and sanitized native metadata. Native lockfiles remain source-owned; Oyzu does not rewrite them to embed localhost ports or tokens. Build preparation can create an isolated normalized view when a manager needs concrete routes, while the persisted plan binds the original lock and logical source identities. Credentials never enter the dependency snapshot hash. Rotation therefore leaves content reuse possible but does not waive authorization.

Authorize access to private bytes on cache reuse and apply offline grant constraints. A shared CAS must not expose private artifacts across tenants merely because digests match. Namespace authorization metadata by tenant/source entitlement, while physical deduplication is an implementation detail inaccessible to unauthorized clients. A revoked source grant denies new use even if content remains retained under another lease.

Execution receives captured package stores read-only or private writable copies, empty credential stores, declared tools and enforced network denial. Native manager offline flags supplement isolation. Snapshot completeness is proven by supported offline execution and tests, not a prefetch command name. Missing build plugins, transitive dependencies or lifecycle assets yield `DEPENDENCY_INPUT_MISSING`, never online retry. Reprepare requires a new final plan with the changed dependency identity.

## Ecosystem obligations

| Adapter | Prepared content | Important independent requirements |
| --- | --- | --- |
| Python | Wheels/sdists, metadata, backend/test dependencies; uv/pip/Poetry-specific offline stores | Target Python ABI/libc, isolated metadata code, editable/path dependencies contained in capture |
| npm/pnpm/Yarn | Exact package archives, manager stores, workspace graph | Native add-on toolchains, lifecycle asset downloads, Yarn PnP, Git source preparation |
| Maven/Gradle | Application, plugin, processor, parent/buildscript artifacts and metadata | Wrapper/JDK verification, included builds and dynamic dependency resolution |
| Ant/Ivy | Explicit recognized Ivy graph and artifacts | Ant itself does not imply resolver semantics; arbitrary get targets fail offline |
| Go | Module graph, archives and integrity records | Private proxy/checksum routing, approved VCS acquisition, cgo compiler/sysroot |
| Cargo | Registry metadata/crates, Git sources, build dependencies | Credential-provider isolation, proc macros on execution platform, build.rs native inputs |
| Helm | Locked HTTP/OCI charts and metadata | Render/package without login files; publication authorization separate |
| apt/apk | Signed repository metadata, exact package closure, installation prerequisites | Distro/arch compatibility, metadata validity, maintainer scripts without egress |

## Errors, cleanup and resource controls

Typed errors include SOURCE_DENIED, AUTH_REQUIRED, LEASE_EXPIRED, ROUTE_UNSUPPORTED, REDIRECT_DENIED, INTEGRITY_MISMATCH, LOCK_CONFLICT, TARGET_INCOMPATIBLE, DEPENDENCY_INPUT_MISSING and RESOLUTION_DID_NOT_CONVERGE. Diagnostics show package/source/phase and remediation without a token or authenticated URL. Retry only transport errors and explicitly retryable upstream statuses, max 3 attempts; permission and integrity failures are terminal. Cancellation closes proxy sessions, invalidates local capabilities and removes temporary auth material before marking acquisition stopped.

Preparation limits inherit the engine file/byte/time bounds. Native metadata and package archives are untrusted data; defend against traversal, decompression expansion and link escapes before capture. Logs are redacted before persistence. Scanners are additional defense; no successful scan certifies absence of arbitrary encoded secrets.

## Acceptance scenarios

- DEP-01: Every supported manager resolves private inputs in standalone and managed profiles without per-project authentication fields.
- DEP-02: Project/dependency code cannot observe upstream tokens through files, environment, IPC or inherited processes.
- DEP-03: Missing dynamic requirements fail offline and never trigger direct internet fallback.
- DEP-04: Captures include build/runtime/test/plugin/native requirements with platform compatibility and verified identities.
- DEP-05: Expired/wrong-tenant/revoked private-content grants deny cache reuse even when bytes exist locally.
- DEP-06: Redirects, embedded lock URLs and alternate protocols cannot escape the approved source routes.
- DEP-07: Interrupted preparation leaves no usable capability or partially committed snapshot; token renewal preserves content identities.

## Implementation sequence and open decisions

Implement typed adapter requests and synthetic authenticated source server, then Python/uv broker acquisition, offline execution and canary tests, then repeat conformance for every manager. Do not build a fake resolver to satisfy expectations. Generate native fixture locks from the synthetic source and retain them as test data. Registry API interoperability and per-platform broker transport are release gates. The chosen endpoint transport implementation must pass IPC isolation and virtualized executor tests; no arbitrary local HTTP listener is authorized by this design. Support versions remain a tested descriptor matrix rather than an unbounded claim of all package managers.

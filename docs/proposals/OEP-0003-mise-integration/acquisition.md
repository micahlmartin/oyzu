# Backend acquisition and authorization contract

Normative draft companion to [OEP-0003](README.md). The production broker's
existing URL authorization and Docker isolation are useful foundations, not a
complete cached-tool authorization service or native installation sandbox.

## Backend admission and reuse

Each compiled admission descriptor identifies canonical backend, allowed tools,
version family, target OS/architecture/ABI, backend compatibility digest, metadata
operations, archive formats, verification kind, layout identity, dependencies,
entrypoint contract and permitted subprocesses. Absence means unsupported.
Descriptors are reviewed release data; a project cannot add or alter them.

| Family | Initial implementation route | Enablement gate |
| --- | --- | --- |
| Core Node | Reuse version metadata, archive selection and environment/layout knowledge; broker SHASUMS/archive/optional publisher verification | Two versions on each admitted native target; digest and signature-strength reporting; no GPG-disable flag inherited from spike |
| Core Go | Reuse backend version rules and archive/layout; replace `git ls-remote` discovery transport with an approved read-only refs/catalog adapter | Exact and range discovery through broker, checksum sidecar, GOROOT and real go invocation |
| Core Java | Temurin only initially; reuse pinned Java metadata interpretation and archive layout | Metadata and archive both authorized, JAVA_HOME and real java/javac on target |
| Core Python | Precompiled PBS only; reuse artifact selection and cryptographic attestation verification | Archive, GitHub attestation and Sigstore trust-material routing; genuine and invalid signatures; writable private verifier cache |
| Aqua | Pinned jq registry entry only; reuse registry interpretation and asset verification | Registry identity, release metadata/archive/checksum and target entrypoint; other registry entries remain disabled |
| Native npm tools | Reuse mise native npm backend with exact locked Node/npm and builder-owned native registry adapter | Full native dependency closure, metadata rewrite integrity, frozen replay, lifecycle admission and Windows typed script launch |
| asdf/vfox and arbitrary scripts | Disabled; no arbitrary plugin fetch/execute | Separate pinned plugin identity, complete outbound inventory, source integrity and qualified script executor amendment |
| Source builds / rustup / other managers | Disabled in this slice | Captured build inputs and qualified executor/manager descriptor; no system compiler fallback |

For core Go, the initial reviewed discovery adapter reads the official Go release
JSON catalog through a logical metadata source, maps its exact versions into the
upstream backend comparator and records the catalog digest. It does not run the
upstream `git ls-remote` discovery path. Tests compare the admitted stable release
set and selected archives against upstream behavior at the pin. Prerelease or
historical versions absent from that catalog fail explicitly until the descriptor
admits another authenticated metadata source; no implicit Git or public fallback.

The experiment establishes only selected Linux backend successes and native Node
lifecycle behavior. It does not enable the rows above in production. Version
discovery and native target layout qualification are implementation work even
where a frozen Linux install already passed.

## Standalone and corporate transport

Use the same typed acquisition boundary in both modes. Standalone supplies
bundled/public source descriptors and makes direct approved upstream requests
without login. Managed supplies verified connector routes from the agent and
prohibits public fallback. The central platform carries policy/credential control
traffic, not mandatory tool bytes. The agent/host broker fetches directly from
the approved upstream or corporate Artifactory/Nexus proxy. On-demand cache misses
are normal; there is no universal pre-mirroring or OCI repackaging requirement.

An acquisition request contains operation ID, logical source/artifact ID,
operation kind (`metadata`, `archive`, `verification`), expected digest/size when
known, method (`GET` or `HEAD`) and a descriptor-defined relative resource key.
The worker never supplies a free-form authenticated URL. A trusted adapter maps
upstream URLs produced by mise into resource keys using parsed origin/path rules;
unmatched URLs fail `TOOL_ROUTE_UNSUPPORTED`. Query fields are allowlisted and
preserved semantically, not stripped to make a route pass.

The broker resolves the logical request against the current route descriptor,
checks operation/tool/version/platform scope and obtains an upstream credential
only on the host. It reauthorizes every redirect, rejects encoded path escapes,
userinfo and credential-bearing query strings, enforces TLS validation, and
strips upstream authentication/cookie/error-body secrets from responses. No
arbitrary CONNECT endpoint. Route failures never change source identity.

Maintain source-specific request and cumulative byte limits. The existing public
Fetcher allows 4,096 requests, 128 MiB per response and 1 GiB per session; these
are measured current limits, not sufficient for every tool. Production introduces
bounded streaming to a broker-owned temporary blob with incremental digest/size
checks, cancellation and backpressure. Defaults for admitted archive sessions are
4,096 requests, 2 GiB per artifact, 8 GiB per session, five redirect attempts and
the operation deadline; metadata remains bounded at 16 MiB per response. The
effective bound is the minimum of descriptor, broker and policy caps. Raising a
cap is an explicit host/admin change, not project input. No unbounded in-memory
response buffering. Test exact boundaries as in the experiment.

HEAD is a real bounded HEAD or a descriptor-declared metadata operation; the
experiment's GET-and-discard emulation is not retained. Range/resume is initially
disabled for artifact downloads: interrupted transfers restart into a new partial
blob within retry/byte budgets. Add resume only after ETag/content identity and
range conformance tests. Retry at most twice for transient transport failures
with bounded backoff; denial, integrity, changed metadata and unsupported routes
are not retryable. An expired lease requires reauthorization, not a new source.

## Native protocol adapters and network enforcement

For built-in library HTTP, the fork uses a broker transport callback rather than
environment-driven URL replacement. The worker may run a session-local loopback
adapter when an unchanged native client needs HTTP. That adapter exists inside
the network-isolated worker and reaches the host only through the private broker
channel. It binds no externally reachable host port, has no upstream credential,
and exposes only that operation's resource namespace. The host channel is mounted
only into the acquisition worker, not into later project execution.

Shared host agent registry endpoints, if used for development package traffic,
must meet OEP-0009's separate local-capability/OS-identity requirements. Do not
reuse the experiment's credential-free host fixture as a production agent API.
Loopback alone is not authorization. No browser-accessible administrative routes
or permissive CORS are allowed.

Npm metadata rewriting changes only admitted tarball transport locations.
Preserve version/dependency/integrity declarations; record both original and
delivered metadata digests. Lock/capture every transitive tarball and native lock
identity before frozen installation; reject Git/URL/workspace forms unless the
native adapter explicitly supports their complete identity. Use exact locked
Node/npm, not globally installed npm. Native lifecycle scripts default off during
capture; packages requiring them fail until a separately captured offline lifecycle
step is admitted. Node scripts get a typed interpreter launch descriptor instead
of a generated `.cmd` wrapper. The dependency-free Prettier experiment is not
proof of this complete npm contract.

The acquisition executor has no external networking, no host HOME, no Docker
socket, read-only root, private writable scratch, bounded resources and a scoped
broker channel. Validate these properties with the actual executor, including a
positive control proving the destination was otherwise reachable. Deny direct
IP/DNS/proxy/Git/curl/package-manager child bypasses. HTTP callbacks alone do not
constitute enforcement. Agent restart, missing image, missing capability or
unavailable isolation fails before acquisition; there is no online host retry.

## Native host prebuilt installation

Support native Windows/macOS/Linux archive tools without pretending a Linux
container can run their post-install programs. Split admitted backends into:
target-aware resolve/acquire/verify in an isolated worker, and trusted data-only
materialization on the host. Add a narrow fork API that returns the upstream
backend's archive/layout/environment facts as an `ArchiveLayoutPlan`. Retain
upstream metadata interpretation; do not implement a second version catalog.
Where upstream install logic is inseparable, that tuple stays disabled until the
reviewed fork exposes the separation and parity tests pass.

Plan fields are `format = 1`, backend descriptor digest, exact target platform,
input blob digests, archive kind (`tar`, `tar.gz`, `tar.xz`, `zip`, `raw`),
single optional strip-prefix, payload subtree, sorted required path/type list,
entrypoint map, install-relative environment path references and extraction
bounds. Optional executable-bit overrides are an explicit sorted path set.
There are no scripts, expressions, arbitrary filesystem destinations, recursive
commands or callbacks. This internal finite record is not a project programming
language. Layout identity is locked and independently checked by the supervisor
against the compiled descriptor; the worker cannot invent a new permitted plan.

For `raw`, the plan has no strip-prefix and exactly one required path of type
`file`; that path names the unchanged blob's destination within the payload.
Parent directories are implicit and consume the usual entry/depth bounds. This
avoids adding a separate destination field or inferring filenames from transport.

Plan entrypoints are templates with `kind`, `payload_relative_path`,
`interpreter_tool_key` (null for native), `interpreter_relative_path` (null for
native) and ordered `prefix_args`. Environment references use `owner` (`self` or
an exact dependency tool key) and `relative_path`; literal values are separately
typed strings. Resolve these to installation keys only after the dependency
closure is prepared. Layout templates never contain their own installation key,
avoiding a circular hash between layout and installation identity. Prefix
arguments may contain typed self/dependency path references, not text substitution
or expressions. The final receipt stores the resolved relative launch records;
the executable operation alone expands them to leased absolute paths.

The host finalizer reads only verified CAS blobs and creates the one staging
payload using the secure extractor from the store contract, reusing audited
upstream extraction utilities where they meet that contract. It performs no
network and spawns no tool code. Known layout transforms outside these fields
require a new reviewed descriptor/API version, not a hidden shell postinstall.
It then creates and atomically publishes the receipt. Test native archive
permissions, internal links, platform-specific directory layout and real execution
on the target before admission. Run publisher signature verification in the
worker before returning the plan; do not downgrade Python provenance or other
verification because host installation is data-only.

The Linux worker profile is provisioned and capability-checked on all hosts using
the current Docker executor; installation never silently installs Docker itself.
If Docker or the appropriate image is unavailable, mediated acquisition fails
with an actionable prerequisite. Already installed, authorized native tools can
still execute locally. A future native acquisition sandbox can replace this
worker only after matching conformance; it is not assumed present in v1.

## Tool-selection authority is distinct from acquisition

Every Oyzu-controlled selection checks both content validity and current
eligibility, including cached exec, shim dispatch, activation and build preflight.
Source URL authorization cannot answer whether an already installed Node version
is currently permitted. Introduce a typed `ToolSelectionAuthorizer` alongside,
not inside, the byte-fetching broker. Standalone returns a local admission
decision after configuration constraints, without contacting a platform.

Managed uses proposed `POST /v1/tool-selections:authorize` through the authenticated
agent. This is a public protocol addition to OEP-0016, not a claim that a private
service already implements it. Request JSON is strict and contains:

| Field | Contract |
| --- | --- |
| `protocol` | Exactly `oyzu.tool-selection/1` |
| `request_id` | UUID, echoed by response |
| `context_id`, `policy_revision` | Verified enrollment/context and current snapshot revision |
| `operation` | `resolve`, `install`, `activate`, `exec` or `build` |
| `selection_digest` | Exact format-2 selection digest; resolve uses a digest of canonical requested constraints and target set instead |
| `subjects` | Sorted exact tool ID/version/backend/platform/distribution/dependency identities, or typed unresolved requests for resolve |
| `capabilities` | Sorted supported enforcement IDs, including receipt verification and selected acquisition mode |

Response contains protocol/request/context, `decision_id`, `decision`
(`allow`, `deny`, `needs-evidence`), `reason_codes`, `policy_revision`,
`selection_digest`, `operations`, `not_before`, `not_after`, `offline_allowed`,
`revocation_epoch` and `grant` on allow. Deny/needs-evidence supplies no grant.
All bindings must equal the requested context/subjects, and a resolve grant never
authorizes later install or exec. Acquisition still requires a separate broker
lease for its exact sources/artifacts. Never send upstream credentials in this
response or persist the grant in a project/lock/bundle.

The [draft signed-payload schema](../../contracts/tools-v1/selection-grant.schema.json)
uses whole Unix UTC seconds for `not_before`/`not_after`, bounded to safe JSON
integers. These proposed wire encodings are not an accepted public API. The schema
covers allow payloads only; deny/needs-evidence responses have no grant. A resolve
payload contains only the resolve operation, and an offline-enabled payload lists
only activate/exec/build; install/discovery require separate online authorization.
Shape validation cannot check lifetime differences, signatures or current context.

Grant payload contains those decision/binding/validity fields plus issuer,
audience `oyzu.tool-selection`, tenant and authenticated subject identity. Use
compact JWS with Ed25519 (`alg=EdDSA`), pinned `kid` from the protected management
trust set, no remote `jku`/`x5u` and no algorithm negotiation. Verify signature,
duplicate-free payload, audience, tenant, subject, context, revision, operation,
selection digest, epoch and time bounds. Unknown fields with mandatory semantics,
unknown keys or algorithms fail. Signing keys are distributed/rotated only through
the protected authenticated management channel; a response cannot supply its own
trust anchor. Public conformance uses synthetic keys, never production credentials.

Online grants have a client-enforced maximum lifetime of 60 seconds. The agent
refreshes in the background while an active session needs them, coalescing requests;
the prompt hook never performs the refresh itself. An allow grant can explicitly
authorize offline local exec/activation/build for at most 15 minutes. These are
proposed conservative release caps, not benchmark-derived claims; policy may
shorten but cannot lengthen them in v1. Offline acquisition is always denied.

The initial verifier interprets these draft caps as follows: an offline-enabled
payload may span at most 900 seconds, but online receipt and online use remain
limited to its first 60 seconds (or its earlier signed expiry). Later offline use
requires the same live verified object. Receipt fixes both monotonic deadlines
from the remaining signed time; changing modes never renews either deadline.
These wire/lifetime choices still require maintainer and service-contract review.

Keep grants only in agent memory in v1. Bind wall-clock validity to a monotonic
deadline when received; backwards wall-clock jumps beyond five seconds invalidate
the context. Agent restart, suspend/resume with uncertain clock, logout, changed
policy revision/epoch or identity loss invalidates all grants and requires online
refresh. Persistent offline grants across reboot are not implemented. No stale
catalog, environment variable, local flag or unsigned cache substitutes for a
grant. This deliberately bounds offline revocation delay rather than promising
instant revocation without connectivity.

The authorizer's local control channel is an owner-only Unix-domain socket with
peer UID verification or a Windows named pipe whose DACL permits only the intended
user and trusted service identity, with remote clients rejected. Bind the request
to the authenticated local session and machine management context; caller JSON
cannot choose another tenant or claim a verified policy revision. Inherited
worker broker handles cannot call this administrative API. Another local user
must fail before request parsing; privileged administrators remain outside this
isolation threat model.

Revocation stops new selections immediately when learned; grant expiry also
blocks new operations. Existing development commands may finish under the grant
checked at launch; this does not promise forcible revocation of arbitrary native
programs already running. Managed build/acquisition supervisors cancel on an
explicit urgent revocation event and stop at grant expiry unless refreshed.
Reauthorization may refresh authority, not change a frozen computation/lock;
changed requirements force replan. A missing agent in an enrolled machine yields
`TOOL_POLICY_UNAVAILABLE`, never standalone routing.

Managed shell activation may publish native tool paths for development, but those
paths do not enforce per-process authorization after export. Policy requiring
per-launch enforcement must select the `mediated-launch` capability: export only
Oyzu executable shims, check grants on every invocation and use qualified executor
actions for protected builds. Even shims cannot prevent a same-user administrator
from manually running copied binaries; OS endpoint controls are outside this
client's authority. Report this threat boundary rather than claiming tool
revocation is operating-system application control.

## End-to-end corporate Node example

For a project requesting Node 22 with no lock, `oyzu install` first reads protected
management state and resolves the effective request without loading mise files.
The agent authorizes discovery for that context. The worker obtains permitted
Node metadata via the logical corporate route and the pinned backend resolves an
exact version/platform. Oyzu verifies metadata/evidence, creates the proposed
format-2 selection and obtains exact install authority plus an acquisition lease.
The broker fetches the archive directly from the configured corporate repository;
its upstream token remains on the host. The worker verifies archive identity and
returns an admitted layout plan. The host finalizer stages, verifies and commits
the receipt; only then does the CLI replace `oyzu.lock` atomically.

On the next `oyzu exec -- node --version`, the CLI recomputes the selection digest,
validates the receipt/tree and current exec grant, leases the payload and launches
its Node entrypoint. No archive request occurs. Changing only the lock digest
produces a different installation key and cannot select the old receipt; command
execution fails until an explicit verified install satisfies the new lock.
Revocation denies a new invocation even with unchanged bytes. An unavailable
corporate route never resolves to a public Node download. Standalone uses the
same flow with local authority and its public source descriptor, without login.

## Diagnostics and retained evidence

Stable diagnostic codes include `TOOL_LOCK_MISSING`, `TOOL_LOCK_STALE`,
`TOOL_LOCK_FORMAT`, `TOOL_LOCK_EDIT_CONFLICT`, `TOOL_DEPENDENCY_CONFLICT`,
`TOOL_PLATFORM_UNSUPPORTED`, `TOOL_BACKEND_UNSUPPORTED`, `TOOL_COMMAND_AMBIGUOUS`,
`TOOL_NOT_INSTALLED`, `TOOL_RECEIPT_MISMATCH`, `TOOL_TREE_CHANGED`,
`TOOL_INTEGRITY_FAILED`, `TOOL_VERIFICATION_FAILED`, `TOOL_ROUTE_UNSUPPORTED`,
`TOOL_SOURCE_DENIED`, `TOOL_SOURCE_UNAVAILABLE`, `TOOL_POLICY_DENIED`,
`TOOL_POLICY_UNAVAILABLE`, `TOOL_GRANT_EXPIRED`, `TOOL_EXECUTOR_UNAVAILABLE`,
`TOOL_LAYOUT_UNSUPPORTED` and `TOOL_CANCELLED`. Records include severity, safe
message, canonical tool/platform, operation ID, source/config span if relevant
and a concrete remedy. Avoid raw URLs/headers/environment values in diagnostics.

Keep local bounded audit records of selected identities, authorization decision
references, route IDs, verification strength, broker response digests, receipt
identity, actual target and outcome. Do not store secret grants, credential values
or arbitrary native stderr without sanitization. Missing/invalid attestation is
not converted into digest-only success. Installation evidence, policy evidence
and runtime test evidence remain distinct so users can see what was actually
verified. Retain failed attempts for diagnosis without treating partial artifacts
as committed installs.

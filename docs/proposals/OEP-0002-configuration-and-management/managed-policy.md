# Managed configuration transport and cache

Status: proposed v1alpha1; part of [OEP-0002](README.md). This extends the configuration portion of [platform protocols](../OEP-0016-platform-protocol/README.md); authorization and connector APIs remain separate.

## Enrollment and standalone operation

Before reading ordinary settings, the CLI reads protected machine records at the [defined locations](implementation.md). A valid `management.json` establishes managed mode independently of login. Standalone mode requires absence of an enrollment record and any provisioned protected management indicator. A malformed, inaccessible or insecure existing record blocks execution. An ordinary `--config`, environment variable or user file cannot hide it.

The bootstrap is a strict JSON object with required fields `schemaVersion` (integer 1), `kind` (`management-bootstrap`), `organizationId`, `enrollmentId`, `platformUrl`, and nonempty `policyKeys`. IDs are opaque nonempty strings of at most 256 characters. The URL must be absolute HTTPS without userinfo, query or fragment. Keys contain `kid`, `kty: "OKP"`, `crv: "Ed25519"`, and base64url `x` representing exactly 32 public-key bytes. Duplicate key IDs fail. Unknown bootstrap fields fail unless inside an inert `extensions` object. Bootstrap contains no refresh token or registry password.

Administrative provisioning writes the record and validates its permissions. Initial key rotation uses administrative provisioning of overlapping old/new public keys, followed by removal after old snapshots expire. A platform response cannot replace its own trust root. Unenrollment removes the protected record through an administrative workflow and clears associated agent sessions/cache; ordinary logout does neither.

Standalone clients work without an account or server. They still verify tool integrity and respect an optional protected `admin-settings.json`. That file is a strict object with `schemaVersion: 1`, `kind: "local-admin-policy"`, `settings`, `profiles`, and `requiredCapabilities`; optional `defaultProfile` and inert `extensions` are permitted. Settings use [policy entries](settings.md). Profile objects contain ordinary default overlays, never constraint objects. Platform-only connector IDs require a resolvable local connector configuration or fail; merely writing an ID does not create a service.

## Policy request and response

The agent reuses `POST /v1/build-contexts:resolve` for applicable build configuration; configuration-only requests use the same endpoint with `purpose: "configuration"`. Requests include supported schema/capability IDs, enrollment and organization IDs, authenticated subject when available, OS/architecture, detected context, repository/workspace selectors and last known revision. Authentication is through the existing platform session or workload exchange; no browser is launched implicitly.

Selectors and CI hints are untrusted observations. The server verifies identity and source claims independently before granting any authority. The request's `configurationContext` is a canonical object containing organizationId, enrollmentId, subjectId (or null), workspaceId (or null), OS, architecture and detected execution class. Its SHA-256 canonical-JSON digest is echoed in the signed response. Repository-sensitive policy additionally requires server-verified repository binding; an arbitrary local workspace ID cannot prove repository identity.

The response adds `configurationSnapshot`, a compact JWS string. Its decoded JSON payload has the following required fields; unknown top-level fields fail outside `extensions`:

| Field | Validation |
| --- | --- |
| `schemaVersion`, `kind` | 1 and `managed-policy` |
| `snapshotId`, `revision` | Nonempty opaque identifiers, max 256 characters |
| `organizationId`, `enrollmentId` | Exact protected-bootstrap match |
| `audience` | Exactly `oyzu-config` |
| `contextDigest` | Lowercase 64-character SHA-256 hex matching the request context |
| `sequence` | Positive monotonic integer per enrollment/context, at most 2^53-1 |
| `issuedAt`, `refreshAfter`, `expiresAt` | UTC RFC 3339 whole-second timestamps; issuedAt <= refreshAfter < expiresAt |
| `offline` | Strict object: `localBuilds` boolean and integer `maxAgeSeconds` between 0 and 86,400 |
| `requiredCapabilities` | Unique array of supported capability strings |
| `settings` | Canonical setting paths mapped to policy entries |
| `profiles` | Named ordinary default overlays |

Optional fields are `defaultProfile` and inert `extensions`. Disabled offline execution requires maxAgeSeconds 0. Enabled offline execution requires a positive maxAgeSeconds. The server's default is 86,400 seconds; administrators may shorten it or disable offline builds. It is explicit signed data, never an assumed grace period when the field is missing.

Policy profile values use the TOML overlay data model encoded as JSON. Dates, arbitrary objects and JSON null are not accepted as ordinary setting values unless explicitly registered. Required arrays/objects may be empty where their contracts permit. Limit signed payload size to 1 MiB, compact envelope to 2 MiB, depth to 32 and entries to the resolver limits.

## Verification

Use a maintained JOSE implementation and published test vectors, not handwritten cryptography. Proposed envelope: compact JWS, protected header `alg: "Ed25519"`, matching pinned `kid`, and `typ: "oyzu-policy+jws"`. Reject other algorithms, duplicate header keys, unknown critical headers, unprotected headers and remote key URLs. Use the fully specified Ed25519 algorithm rather than negotiating algorithms from the payload. JSON payload bytes use JSON Canonicalization Scheme; verify signature over the original JWS signing input before trusting payload fields, then validate canonical encoding and schema.

Verify the pinned key, organization/enrollment, audience, expected context, capabilities, timing, sequence and constraint consistency before exposing `VerifiedPolicySnapshot`. Signature validity alone does not establish applicability. Resolve machine-only/anonymous context separately from authenticated-user context; do not reuse another user's cached snapshot after logout or account switch.

Reject issuance more than 120 seconds in the future; this tolerance does not extend expiry or offline deadlines. A strictly lower sequence than the recorded high-water mark is rejected. An equal sequence is accepted only for identical payload digest. New snapshots, including renewals of unchanged policy, use a higher sequence. Signature keys and bootstrap identity changes invalidate incompatible caches.

The proposed standards are [JCS](https://www.rfc-editor.org/rfc/rfc8785.html), [Ed25519 JWK representation](https://www.rfc-editor.org/rfc/rfc8037.html), and [fully specified JOSE algorithms](https://www.rfc-editor.org/rfc/rfc9864.html). Library interoperability is a release gate.

## Cache and refresh lifecycle

Store exact signed envelopes under user state, partitioned by organization, enrollment and context digest. Never cache reusable secrets inside the policy payload. The headless agent owns updates; the CLI can start/connect to it without a desktop. Readers validate signatures and applicability even when a file is already named as verified.

Write a new immutable entry, flush it, then atomically replace the active pointer. Serialize concurrent refreshes per context. Preserve the prior verified entry until the new entry is durable; disk-full, truncation or crash cannot turn partial data into active policy. Protect directory permissions and keep a high-water sequence and last validated clock observation in integrity-protected OS-backed state. If this state is missing or inconsistent after prior enrollment, require online reconciliation before offline execution. Do not infer freshness from filesystem timestamps.

Same-user processes and local administrators may be able to access or replace user-owned state. Integrity-protected local state detects ordinary corruption and observable rollback; it is not an absolute anti-rollback guarantee against a hostile OS principal. Strong authority still comes from the server and privileged services. A deployment requiring stronger offline anti-rollback must add an OS-protected service or hardware-backed mechanism before claiming it.

Refresh at the signed refreshAfter time; the server should normally choose roughly 15 minutes after issuance with distributed jitter. Coalesce concurrent requests. On transport errors use bounded exponential backoff from 5 seconds to 5 minutes with jitter, capped by the remaining valid interval. A foreground explicit refresh reports its outcome. HTTP cache metadata and 304 responses never renew signed issuance or expiry; extending validity requires a newly signed snapshot.

Freeze one applicable snapshot for a plan. A newer policy can block a subsequent privileged action through fresh authorization, but does not mutate recorded inputs. If new checks are required, replan. During valid offline use, immediate remote revocation cannot be guaranteed; the configured validity window is the explicit tradeoff.

## State and operation rules

| State | Behavior |
| --- | --- |
| No protected enrollment | Standalone defaults plus any protected local administration |
| Valid enrollment, no applicable snapshot | Help/status/login/refresh available; policy-dependent operations blocked |
| Verified current snapshot, online | Execute allowed operations; privileged grants remain separately authorized |
| Refresh transport failure, snapshot still usable | Permit only expressly covered offline operations; report cached revision and deadline |
| Explicit server denial/revocation | Block affected operations; never disguise denial as an offline transport failure |
| Expired, mismatched, corrupt or incompatible snapshot | Block affected execution and request refresh/upgrade; never public fallback |
| Explicit logout/account switch | Clear session grants and active user binding; do not reuse the prior user's snapshot |

A separately verified machine-only snapshot may permit local work without user login. Absence of one does not cause downgrade. Passive session expiry can use a still-applicable cached local-build snapshot bound to the same OS user and recorded subject, but cannot obtain new credentials; explicit logout removes that subject binding.

For a local build offline, the deadline is `min(expiresAt, issuedAt + maxAgeSeconds)`. The snapshot must explicitly enable offline local builds, match the current context, and permit every required operation. If the trusted time estimate cannot establish freshness, fail with `POLICY_CLOCK_UNCERTAIN`. Compare monotonic elapsed time during a running session and persist the last validated wall-clock observation; suspicious backward movement requires online reconciliation. These checks cannot defeat an administrator controlling the system clock and storage.

Offline builds can consume already installed approved tools and dependencies available in approved local caches. Missing inputs fail with an actionable list; do not fetch public substitutes. A still-valid, independently scoped credential lease may continue an operation only if its own authorization explicitly permits it and the upstream remains reachable; that is not a grant from cached policy. Initial offline-build support should require no new credential issuance.

Signing, publishing, promotion and obtaining new protected credential grants require fresh online authorization bound to the relevant subject, operation, destination and artifact/source evidence. Inferred CI alone is insufficient. The initial 24-hour offline allowance applies to local builds, not CI builds; managed CI requires an online context resolution at start. Non-executing inspection and cache diagnostics remain available in every state with redaction.

## Operational verification

Test first enrollment, anonymous versus subject-bound cache, logout, renewal, offline deadline equality (expired at deadline), shorter administrator limits, missing offline fields, negative server responses, signature/key mismatch, unknown capabilities/operators, wrong audience/context, revision rollback, equal-sequence changed payload, truncated writes, concurrent readers, disk-full, clock rollback, reboot and OS permission failures. Publication/signing tests must prove denial even when CI variables and profile names are spoofed.

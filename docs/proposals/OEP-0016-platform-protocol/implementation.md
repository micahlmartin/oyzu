# Build policy and CI integration implementation contract

## Context resolution

The build client calls a common context provider in standalone and managed modes. Standalone returns local defaults plus native CI observations, without contacting Oyzu accounts. Managed mode first verifies machine/runner management selection, identity/session and tenant scope, then obtains a compatible policy snapshot. Product entitlement is distinct from management: entitlement failure cannot switch to standalone routing.

CI provider adapters normalize provider, repository, commit, event, ref, workflow identity, fork status, run id and protected-ref observations. Environment values are hints until verified. Verified workload identity must bind issuer, audience, subject, tenant, workflow/source and freshness to the captured invocation. An unsupported CI provider can still run local-equivalent builds but cannot self-assert production eligibility. A pipeline need only invoke Oyzu; existing CI is a trigger/executor, not a source of hand-authored scanner/cache stages.

## Build-specific request and decision records

Initial versioned resources are `POST /v1/build-contexts:resolve`, `POST /v1/build-decisions:evaluate`, `POST /v1/acquisitions:authorize`, `POST /v1/publications:authorize`, and `POST /v1/evidence:record`. These are proposed public paths to implement behind the same typed client interface; auth bootstrap remains separately versioned. Each request includes protocol version, request/correlation id, tenant/context id and supported enforcement capabilities. No upstream credentials appear in build-decision payloads.

Context resolution returns immutable snapshot id/revision, issued/expiry times, audience/tenant/context binding, defaults, mandatory constraints, connector route references, required checks and verification metadata. Decisions return decision id, allow/deny/needs-evidence, requested operation, scope, reason codes, required evidence references and validity. Acquisition authorization returns a broker-consumable lease reference; project processes cannot call the API to obtain an upstream secret. Signing/publication authorization binds final subject digests and destination scope; never accept `trusted=true` as proof.

## Required checks and profiles

Policies select checks by artifact kind, source ownership, invocation context, classification and supported capabilities. Each RequiredCheck is data: id, versioned adapter, prepared tool/rules/database identities, subject binding, parameters validated by that adapter, pass/fail thresholds, evidence freshness and whether reuse is allowed. No arbitrary shell code or policy-programming language is delivered to the CLI. New scanners require a registered adapter with a supported descriptor; unknown mandatory adapters cause an actionable upgrade/capability failure.

Local defaults include native compile/test, configured lint/format checks and inexpensive metadata collection. CI policy can add SAST, dependency/SBOM, license, vulnerability, image and artifact-integrity checks without editing the repository. Select scanner/database identities during preparation; a live database changing mid-run requires a new check input and evidence record. A format task never rewrites the user's sources as a hidden build step. A fast local profile can omit optional checks but must disclose them and cannot satisfy a policy requiring those checks.

Every check is bound to source or exact produced artifact digest, not an unrelated workspace. Threshold evaluation belongs to the versioned check adapter and policy; retain normalized findings and native reports. Missing/unsupported/invalid reports fail mandatory requirements. A stale clean report cannot satisfy a current database/freshness requirement merely because a cache key matches compilation.

## Freshness, offline and reauthorization

Freeze one effective policy snapshot for a plan. Dispatch checks that action grants remain within expiry and scope; renew authorization without mutating computation if requirements are unchanged. Changed requirements trigger replan before affected work, never silently edit a frozen plan. Explicit urgent revocations halt newly unauthorized work and block publication; inability to learn revocations offline is bounded by the grant's validity.

Offline managed default is deny new protected acquisition/publication. A previously verified snapshot may explicitly permit local execution and private cache reuse until its notAfter; the engine checks subject/tenant/tool/source scope and clock validity. Detect significant clock rollback using stored observation bounds and fail offline authorization if validity cannot be established. Offline grants require authenticated cached metadata and tested key rotation; until implemented, managed offline mode is unavailable rather than accepting unsigned config. No universal arbitrary number of grace hours is assumed.

Optional telemetry uses bounded redacted queues and never fails a standalone build. Mandatory managed evidence delivery is explicit in policy: block the privileged operation if acknowledgement is required and unavailable, while retaining the finalized local bundle. The platform does not need to stream dependency bytes to enforce these rules; upstream leases and connector permissions provide independent enforcement.

## Verification

PROTO-01–06 plus forged CI variables, wrong audience/tenant, signed-but-expired snapshot, changed scanner requirement without repository change, required unknown adapter, interrupted evidence submission, clock rollback and policy revocation between build and publication. The public harness uses synthetic provider/identity facts and keys. Real CI identity and signing integrations must pass their provider-specific suites before authority is enabled; these endpoints do not imply private server implementation is complete.

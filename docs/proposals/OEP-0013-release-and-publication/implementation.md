# Version, evidence and publication implementation contract

## Classification and facts

Use classifications local, snapshot, candidate and release as policy outputs; they are not CLI privileges. Local invocation is local-origin even on a protected branch. CI detection selects a provider adapter but starts unverified. Facts are true/false/unknown with source, subject repository/commit/ref, observation time, validity and verification material. Record workflow source/ref, fork context and checked-out commit separately. GitHub/GitLab protection APIs are supporting facts; a protection flag alone never authorizes production.

Local source identity is the captured tree digest including permitted dirty/untracked inputs. Clean Git commit identity binds the exact checkout and submodule closure. Changes between verified CI event and captured source yield a mismatch, not an automatic release. Detached HEAD, shallow history, missing permissions, deleted tags and inconsistent APIs have typed unknown/error states. Required unknown facts produce needs-evidence/deny. Never synthesize protection from branch naming.

## Deterministic version strategy

Native declared versions remain the base. The initial strategy id is native-source-v1. For local/snapshot artifacts define `sourceSuffix = first 12 hex characters of captured source tree digest`, with the full digest retained to detect coordinate collisions. Stable release/candidate versions must be declared or derived from an authorized exact tag mapping; no automatic guessed version bump. Existing prerelease identifiers are preserved when policy permits and otherwise require an explicit version choice.

| Artifact | Development version from base 1.2.3 | Release behavior |
| --- | --- | --- |
| Python wheel/sdist | 1.2.3.dev0+g<sourceSuffix> | Native PEP 440 version; destination must support the chosen form |
| npm, Cargo package, Helm | 1.2.3-dev.g<sourceSuffix> | Native valid SemVer; package/chart internal metadata must agree |
| Maven/JVM artifact | 1.2.3-dev.g<sourceSuffix> | Fixed immutable coordinates; native -SNAPSHOT behavior is a separate explicit strategy |
| OCI image | 1.2.3-dev.g<sourceSuffix> tag | Tag is only a pointer; manifest/index digest is identity |
| Go executable | Bundle version 1.2.3-dev.g<sourceSuffix> | Binary metadata only where supported; do not fabricate a Go module release |

Adapters apply version changes in isolated packaging metadata/worktrees, never the checkout. If an adapter cannot safely reconcile versioned native dependencies, fail with the needed explicit project change. Coordinated native module versions are preserved; independent modules retain their own base versions. An OCI/chart may have different versions while digest binding records their relationship. Native Maven timestamped SNAPSHOT repositories are not claimed immutable: their explicit adapter must bind resolved coordinates and bytes, and cannot use overwrite semantics for protected releases.

The default is a reproducible identity mapping, not a monotonically ordered nightly-version scheme. Destinations that reject local/dev metadata need a policy-selected supported strategy, not lossy string stripping. Persist strategy id/version, base, mapped versions and source identity in the plan. [Python version rules](https://packaging.python.org/en/latest/specifications/version-specifiers/) are applied by a conforming parser, not a handwritten universal SemVer parser.

## Eligibility gates and policy inputs

Preflight validates invocation eligibility; planning selects required checks and candidate coordinates; execution records achieved facts; publication asks again with the final artifact/manifest digests. Policy maps artifact kind, repository/project, origin, classification and verified facts to registry connector, repository, immutable coordinate constraints, required scans/thresholds, signature authority and attestations. Unsupported mandatory checks deny planning. A developer override can change implementation but cannot turn a required check off.

Managed production gate: nonlocal verified execution identity, bound approved source, clean captured content, required source-protection/tag facts, authorized dependency/tool inputs, sufficient enforced isolation, passed mandatory checks over the same digest/source, acceptable cache-producer chain and fresh operation authorization. These are evaluated facts, not different trusted builders. A reused local-origin output remains disallowed as production build evidence. A true rebuild on verified CI can create new nonlocal output/evidence; merely re-signing or repackaging local output cannot.

## Publication transaction and retries

`oyzu publish <bundle>` verifies the finalized manifest and requested artifact bytes before obtaining a short-lived operation grant. `--destination` is a requested configured route, never permission. Resolve every operation into an immutable PublishPlan with source manifest digest, artifact digest, target coordinate, signing/attestation requirements and idempotency key. The key is SHA-256 of canonical operation kind, artifact identity/digest, destination identity/coordinate and policy-required output type, excluding token values and attempt timestamps.

Per operation: planned → authorized → uploaded → verified → attested/signed where required → completed. Failure records retryable/nonretryable and the last confirmed boundary. Write a separate receipt under an operation output directory outside dist. On retry, query the destination and compare bytes/digests: identical content satisfies upload, absent content retries, conflicting content fails. Never overwrite a release version to make a retry succeed. Reauthorize before each privileged retry; an expired grant is not reused because an operation id matches.

A multi-registry release can partially succeed. Default stop on first failed dependent publication, retain independent successes and produce a failed/partial receipt. Rollback is not implied; deletion requires separate authorization and may be impossible. Promotion copies or retags the same verified bytes and checks registry digest preservation. Signing authority calls bind exact subjects and current eligibility; production and nonproduction authority scopes are distinct.

## Provenance and attestations

Generate in-toto statements with the supported SLSA provenance predicate via a versioned adapter, keeping subject digests, build definition and run details consistent with the bundle. The signing identity must be verified independently of unsigned client fields. Record materials/resolved dependencies, external parameters, builder implementation identity, invocation id and actual checks with safe references. A secret or signed acquisition URL is never a provenance parameter. SBOM formats are selected by artifact adapter and policy, initially SPDX JSON or CycloneDX JSON through pinned generators.

Do not claim a SLSA level from the existence of a file. Each level assertion needs the applicable producer controls and verified evidence. Follow the [SLSA provenance specification](https://slsa.dev/spec/v1.2/provenance) and record the exact supported predicate version. Registry referrers may attach evidence; unsupported registries use a connector-supported companion artifact/receipt reference without pretending tags are immutable.

## Verification and remaining provider gates

REL-01–06 plus table-driven version grammar tests, dirty-tree collision, native multi-module version mismatch, expired facts, fork PR identity, policy revision changed after build, lost upload response, partial multi-registry success, registry digest mutation and fake signing claims. Provider token-exchange, keyless/KMS signing integrations and source-protection mappings require real compatibility tests before enabling production use. No design document can substitute for those trust checks.

---
id: VIS-007
status: draft
updated: 2026-10-01
---

# Artifacts caching and evidence

> Vision draft consolidating agreed product direction. No feature is implemented by this document. Unresolved implementation choices are identified explicitly.


Every build should produce a self-describing bundle that downstream tools can inspect without rebuilding or guessing artifact paths.

## Bundle model

`dist/manifest.json` indexes the plan, target outcomes, artifacts, test results, coverage, validation, and provenance. Paths are bundle-relative and content references include digests. Failed and interrupted builds retain partial evidence where the filesystem is writable. Stale output from another invocation must never be mistaken for current output.

The build manifest becomes immutable at finalization. Publication receipts and later policy evaluations are separate records that reference its digest.

## Caching

Local caching is automatic. Standalone remote caching can use a configured OCI registry without a custom service API. Builders identify safe inputs/outputs and the engine manages keys, lookup, restoration, and concurrent writes.

Cache entries retain producer identity and trust information. A checksum authenticates bytes only against a reference; it does not establish that a producer executed a valid action. Production builds cannot consume arbitrary developer-produced action results as trusted work.

Mutable package-manager scratch caches differ from authoritative action results. Scan results also depend on external database freshness. Registry cache permissions must prevent untrusted writers from poisoning trusted results.

## Versioning and publication

Builders identify artifact types and existing project versions. Policy or standalone configuration selects version conventions and destinations. Exact versions and classifications are frozen before packaging; artifact digests are known after execution.

Local-origin artifacts remain ineligible for managed production publication. Copies, new tags, signatures, and uploads from CI do not upgrade that origin. Signing and publication bind authorization to artifact digests and verified evidence.

Python package versions, chart versions, application versions, and container tags are distinct values. Promotion preserves bytes only when the ecosystem permits it. Changed package metadata creates a new artifact.

## Evidence

SBOMs, test/scanner results, build provenance, and release authorization are distinct claims. Signing authenticates the issuer, not the truth of arbitrary client claims. Required SLSA guarantees depend on execution/provenance controls, not a manifest checkbox.

## Success

A copied bundle can be verified and published without the source checkout. Repeated publication is resumable and cannot overwrite a different immutable artifact. Cached work is explicitly attributed. Sensitive information is excluded or access-controlled rather than blindly exported with a public release.

## Open questions

Exact schema fields, OCI cache record layout, evidence retention periods, and per-ecosystem signing mechanisms remain proposed in the associated OEPs.

---
id: OEP-0012
title: Build bundles manifests and reports
status: draft
implementation: in-progress
updated: 2026-10-01
authors: [micahlmartin]
reviewers: []
requires: [OEP-0006]
tracking-issue: null
---

# Build bundles manifests and reports

> Design draft for review. MUST and SHOULD express proposed normative requirements, not shipped behavior. Example syntax and protocol fields are provisional unless identified as an agreed product constraint.

Implementation detail: [v1alpha1 implementation contract](implementation.md). This companion resolves the initial engineering defaults below; it remains draft, and does not imply maintainer acceptance or verified platform support. Where an older paragraph leaves an implementation choice open, the companion is the proposed initial resolution.

## Problem and outcome

Every build should produce a portable `dist/` bundle describing what happened and what was produced. Publishing, inspection, signing, and attestations should consume that bundle rather than scrape logs or reconstruct intent.

## Bundle structure

Proposed layout:

```text
dist/
  manifest.json
  artifacts/<target>/<variant>/...
  reports/<target>/<variant>/...
  evidence/...
```

The engine assigns collision-free relative paths. Multiple concurrently running builds use isolated staging directories; a successful finalization atomically selects the final destination or reports an explicit destination conflict. It never mixes files from two runs. Preservation of previous runs and a configurable output root are optional follow-up UX choices.

The manifest has a schema version and records:
- Build/run identity, semantic plan digest, source revision and source content digest, dirty state, and execution context.
- Builder/tool versions, relevant lock/input digests, policy revision/decision references, and capabilities actually enforced.
- Targets, variants, effective tasks/hooks, action outcomes, timings, cache-hit origins, and dependency relationships.
- Artifacts with logical identity, type, version, media type, path, size, content digest, and producing action.
- Test/coverage/security reports with format, target/action association, path, digest, summary, and completeness.
- Evidence references, external-input disclosures, errors, skipped work, and aggregate outcome.

This list defines semantics; a machine-readable JSON Schema must be introduced with the implementation and conformance fixtures before schema v1 is stabilized. Unknown required fields or unsupported major versions must fail safely. Consumers may ignore documented optional extensions without changing interpretation of required evidence.

## Reporting contract

Builders discover native test/coverage facilities and request machine-readable output automatically. JUnit XML and ecosystem coverage formats are collected without ordinary project configuration. Oyzu preserves original reports and normalized summary metadata; it does not invent successful tests or coverage percentages where none were produced.

No tests detected, tests skipped, zero tests collected, test runner failure, missing report, and successful tests are distinct states. Coverage is enabled when the builder supports a deterministic integration. Missing reporter dependencies must be resolved as pinned preparation inputs, not installed ad hoc during tests. Unknown custom runners report unsupported automatic collection and offer a narrow override contract.

An explicit task override inherits the required report contract. If it fails to produce a required report, the build explains the mismatch. Policy determines whether optional coverage is informative or mandatory; absence cannot be treated as passing a threshold.

## Integrity and lifecycle

Artifacts are hashed after producers finish. The engine validates paths, rejects escaping links, and records byte sizes. Paths are portable and relative; credential values, home paths, private tokens, and unnecessary environment dumps are excluded.

The engine writes an incremental journal during execution and finalizes a manifest on success or handled failure. A crash before finalization leaves a recognizable incomplete run, never a success manifest. Failed bundles retain available diagnostic reports subject to redaction and retention rules.

A manifest cannot include a digest of itself. Signatures and attestations that cover the finalized manifest are separate envelope files or registry objects. Generated evidence binds to artifact/manifest digests, avoiding a circular hash relationship.

Finalized bundle contents are immutable. Publication produces a separate receipt mapping artifact digests to destination coordinates, registry-returned digests, authorization/evidence references, and operation status. A receipt never upgrades the original execution facts.

## Consumption and compatibility

Publishing and signing validate all referenced bytes and required reports before use. Copies across machines remain verifiable. Transport archives preserve the manifest contract. Registry artifacts can carry the manifest or refer to its immutable digest.

A partial build may have usable development artifacts, but its aggregate failure and missing requirements remain visible. Production publication requires complete policy-compliant evidence for the relevant outputs and their dependency closure.

## Acceptance scenarios

- BUNDLE-01: A multi-target build emits distinct artifacts and reports with digest-verifiable paths.
- BUNDLE-02: Tests failing still produce a failed manifest and any available reports.
- BUNDLE-03: Missing coverage is distinguishable from zero coverage and successful collection.
- BUNDLE-04: Mutating an artifact after finalization causes publishing validation to fail.
- BUNDLE-05: Publishing adds a receipt without modifying the original manifest.
- BUNDLE-06: A crash or output-directory race cannot produce a false complete bundle.

## Open decisions

Finalize schema, report normalization, output-root/retention UX, large-log storage, and source path redaction. Define schema migration and compatibility fixtures before stabilizing consumers.

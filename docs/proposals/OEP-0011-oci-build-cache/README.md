---
id: OEP-0011
title: Local and OCI remote build caching
status: draft
implementation: not-started
updated: 2026-10-01
authors: [micahlmartin]
reviewers: []
requires: [OEP-0006, OEP-0007]
tracking-issue: null
---

# Local and OCI remote build caching

> Design draft for review. MUST and SHOULD express proposed normative requirements, not shipped behavior. Example syntax and protocol fields are provisional unless identified as an agreed product constraint.

## Problem and outcome

Build caching should be automatic, with no developer-maintained CI cache keys or cache service. Standalone users can configure an OCI registry; managed users receive cache destinations and permissions from policy. Local caching remains useful and enabled where safe.

## Cache layers and identities

Distinguish tool downloads, dependency content stores, action results, and final distributable artifacts. They have different retention and trust properties. A dependency download cache is not evidence that compilation or tests ran.

An action key covers canonical command/task definition, hooks where their effects belong to that action, input digests, toolchain and builder versions, executor compatibility, relevant environment, variants, and policy constraints affecting execution. Host paths, credentials, timestamps, and unrelated policy text are excluded. Eligibility decisions remain outside the reusable computation key when they do not alter computation.

Outputs are content addressed. An action-result descriptor maps the action key to output digests, report references, execution properties, and producer evidence. Missing or unverifiable fields yield a miss. Cache lookup never substitutes for mandatory preflight or current release authorization.

## OCI storage contract

Use OCI content blobs and manifests with an Oyzu artifact type. A stable locator derived from the action key discovers the result descriptor. The descriptor references immutable output blobs. Tag lookup is a discovery mechanism; the client verifies the resolved descriptor and every digest rather than trusting the tag name.

Upload blobs first and commit the manifest/locator last. Multiple writers of an identical action key may race. Identical results converge; incompatible output digests for the same key are recorded as a reproducibility conflict and MUST NOT silently replace trusted results. Registries without conditional tag writes require immutable candidate identities and a verification rule; the exact locator scheme needs an interoperability prototype.

Support OCI registries without requiring the referrers API as a universal prerequisite. Registry-specific authentication and feature differences are connector capabilities. Retention and garbage collection are explicit; cache eviction must not delete artifacts required by published release records.

A proposed standalone configuration identifies only destination and access preference:

```toml
[cache]
remote = "oci://registry.example.com/team/oyzu-cache"
```

No custom cache API, per-language cache path list, or hand-authored CI restore/save step is required. Managed configuration supplies equivalent effective settings within policy constraints.

## Trust and local behavior

A local developer can read an authorized shared cache, keep a private local cache, and optionally write a development namespace. Policy controls those choices. Trusted CI MUST NOT accept locally produced results as production evidence merely because their bytes were uploaded to the same registry.

Trust evaluation uses producer identity, execution evidence, source/input identity, and verified cache metadata. Cache hits retain the original producer/evidence chain; the consuming CI run cannot claim it executed a cached action. A trusted producer's signed descriptor does not automatically make every result acceptable for every policy.

Hooks with external effects and secret-dependent commands are noncacheable by default. Native nondeterministic tasks also start noncacheable until their inputs/outputs are understood. If a cacheable main task is skipped, separately planned noncacheable hooks still run according to task semantics.

## Failure handling and performance

A missing/unavailable cache normally falls back to computation, provided dependencies and execution prerequisites are available. Authorization failures never trigger writes to a public substitute. Corruption causes quarantine or rejection and a fresh computation; mandatory evidence violations may fail the build.

Local stores use atomic writes and concurrent-process locks. Remote transfers are bounded, cancellable, deduplicated, and lazy where supported. Measure metadata round trips, blob count, compression cost, and registry limits. A future enterprise cache service can implement the same action-result contract without becoming an OSS prerequisite.

## Acceptance scenarios

- CACHE-01: An OSS CI build uses only an OCI registry and automatic action-key generation.
- CACHE-02: Toolchain, hook, declared environment, and source changes invalidate affected actions.
- CACHE-03: Interrupted/racing uploads cannot expose partial or silently conflicting trusted results.
- CACHE-04: A local cache upload cannot satisfy a trusted CI producer requirement.
- CACHE-05: Corrupt blobs are rejected and cache eviction does not alter release artifacts.
- CACHE-06: A cache miss or permitted outage rebuilds without pipeline changes.

## Open decisions

Prototype artifact media types, immutable candidate discovery, compression/chunking, retention, registry limits, and first supported registries. OCI is the default interoperability choice; no performance equivalence to a dedicated cache service is assumed.

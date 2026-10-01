---
id: OEP-0013
title: Versioning release eligibility and publication
status: draft
implementation: not-started
updated: 2026-10-01
authors: [micahlmartin]
reviewers: []
requires: [OEP-0010, OEP-0012]
tracking-issue: null
---

# Versioning release eligibility and publication

> Design draft for review. MUST and SHOULD express proposed normative requirements, not shipped behavior. Example syntax and protocol fields are provisional unless identified as an agreed product constraint.

## Problem and outcome

Versions, destinations, signatures, and attestations should follow discovered facts and policy. A developer must not turn a local build into a production release by supplying a flag or uploading it from CI.

## Separate decisions

The planner computes an artifact version and a proposed build classification. A policy decision determines eligibility for specific operations: snapshot publish, release publish, nonproduction signing, production signing, and promotion. Version text, eligibility, and actual publication state are distinct.

The decision inputs include verified source repository and commit, ref/event context, protected-ref facts, tag provenance, dirty state, execution identity, enforced isolation, required checks, original cache producers, dependency evidence, and effective policy revision. A protected branch alone does not establish all release requirements. API failures produce unknown facts, not positive eligibility.

Provider adapters query supported source-control APIs where needed. GitHub/GitLab semantics are normalized without pretending they have identical protection models. A local clone cannot self-assert server-side protection or trusted CI identity. CI environment-variable detection identifies an adapter candidate; verified workload identity and source bindings establish trust.

## Version calculation

Prefer native project metadata and ecosystem conventions. Determine one version per artifact/variant before execution and fail incompatible or conflicting source versions. Snapshot versions derive from declared base version plus stable source identity using ecosystem-valid encoding. Do not use wall-clock timestamps by default when they would make equal inputs produce different versions.

Examples must cover Python PEP 440 versions, npm/Helm SemVer conventions, Maven snapshot/release coordinates, OCI tag restrictions, and Go/Rust native package semantics. One universal version string cannot be blindly used for every artifact kind. A logical release identity can map to ecosystem-specific versions, with the mapping recorded in the plan.

Dirty local builds need content-derived identity and are ineligible for production. User-supplied version overrides are requests subject to policy and collision checks, not grants of release authority. Multi-module projects may use coordinated or independent versions inferred from native metadata; an ambiguous strategy requires a minimal explicit choice.

## Publication and promotion

A proposed publish operation reads the immutable bundle, verifies digests and required evidence, and obtains a current scoped authorization for the selected destination. Policy maps artifact kind, classification, source, and environment to connector/repository, signing authority, and attestation requirements.

Upload content to immutable coordinates where supported. Re-running a successful publish is idempotent when destination content matches; conflicting content fails. Multi-artifact publication may partially succeed across registries, so a receipt records every operation and retry state. Atomic cross-registry transactions are not promised.

Promotion references the same verified artifact digest; it does not rebuild silently. Whether copying between registries preserves the required digest is verified per media type/registry. Tags remain pointers, not identity. A local-origin artifact remains local-origin after upload, scanning, or a later CI signature, and is denied production promotion under the agreed policy boundary.

Signing happens only after digest and eligibility verification. Separate production/nonproduction authorities and scopes prevent an ordinary developer token from requesting production signing. Attestations identify their producer, subject, predicate, and verification material. A generated JSON document alone is not trusted provenance.

## Policy and evidence freshness

Execution records the policy used to plan and run. Publication reauthorizes against current release policy and revocations, recording both revisions. New policy may require rebuilding or collecting additional independently valid evidence; the system cannot rewrite historical facts.

Claims about SLSA levels require the actual producer controls and provenance requirements of the applicable specification. Oyzu should report verified properties and identified gaps, not advertise a level because an attestation file exists. See [SLSA build track](https://slsa.dev/spec/v1.2/build-track-basics).

## Acceptance scenarios

- REL-01: A local artifact on a protected branch cannot be published/promoted to production.
- REL-02: Spoofed CI variables or a release-looking version cannot confer signing authority.
- REL-03: Equal source/version inputs produce consistent ecosystem-valid planned coordinates.
- REL-04: Re-publish with matching bytes is idempotent; conflicting bytes fail.
- REL-05: Changed release policy is evaluated at publish time without rewriting execution history.
- REL-06: A cached output retains its original producer evidence through signing and promotion.

## Open decisions

Specify initial version strategies, provider evidence schemas, signing providers, attestation formats, registry atomicity limitations, and the exact production evidence requirements before claiming release support.

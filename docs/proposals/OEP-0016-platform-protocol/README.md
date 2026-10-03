---
id: OEP-0016
title: Public platform protocol and policy decisions
status: draft
implementation: not-started
updated: 2026-10-03
authors: [micahlmartin]
reviewers: []
requires: [OEP-0002, OEP-0010]
tracking-issue: null
---

# Public platform protocol and policy decisions

> Design draft for review. MUST and SHOULD express proposed normative requirements, not shipped behavior. Example syntax and protocol fields are provisional unless identified as an agreed product constraint.

Implementation detail: [v1alpha1 implementation contract](implementation.md). This companion resolves the initial engineering defaults below; it remains draft, and does not imply maintainer acceptance or verified platform support. Where an older paragraph leaves an implementation choice open, the companion is the proposed initial resolution.

The [account access context and permission-check companion](access-context.md) defines the proposed public membership context, safe checks and scoped-access administration slice. It remains draft and is separate from tool acquisition and credential-lease behavior.

## Problem and outcome

A public CLI must remain usable independently while supporting a stable enterprise control-plane contract. Login, management state, entitlement, policy, and upstream authorization are different concepts and must not be conflated.

## Public protocol resources

The versioned protocol covers:
- Identity/session bootstrap and verified workload identity exchange.
- Organization membership and product entitlements.
- Effective management configuration and policy snapshots.
- Filtered tool catalogs and exact acquisition authorizations.
- Connector/route descriptors and scoped credential leases.
- Build-context facts, policy evaluation, and publish/signing authorization.
- Optional audit/evidence submission with declared retention and privacy boundaries.

Public contract fixtures use synthetic organizations, endpoints, and credentials. No private server code is required to build/test the CLI. A mock conformance server can validate the protocol.

Proposed endpoints use a versioned API namespace, but concrete paths and JSON schemas remain to be finalized alongside an implementation spike. Each resource has stable identity, schema/version information, expiry where applicable, and error semantics. Never return a secret where a reference suffices.

## Bootstrap and management

The protected machine settings specify management requirement, platform URL, organization scope, and trusted bootstrap information. The CLI reads them before network acquisition. Standalone users need no account. Managed users without a valid session receive an authentication/recovery path and cannot silently become standalone.

Authentication proves subject identity; membership establishes tenant context; entitlement enables paid service features; policy and upstream scope authorize individual operations. Subscription failure cannot disable management constraints or send a workstation to public sources. Define read-only/grace behavior explicitly without manufacturing authorization.

There is no custom device certificate enrollment prerequisite. Future posture/device-attestation capabilities require separate design and cannot be retroactively implied by the word “managed.”

## Policy snapshot and decisions

A snapshot identifies tenant, subject/context scope, schema and policy revision, issuance/expiry, defaults, mandatory constraints, and the rules/provenance needed for explanation. Authenticity uses authenticated transport plus verifiable cached metadata appropriate to the threat model. Key rotation and trust anchors must be specified before offline policy caching ships.

Client-side evaluation improves UX and local enforcement. The server independently evaluates privileged token, publication, and signing requests. It never trusts a client-supplied “release=true” or unsigned summary as evidence.

Decisions include allow/deny/needs-evidence, permitted operation and scope, reason codes, relevant policy revision, expiry, and required evidence references. Unknown facts and unsupported client capabilities are first-class. A planner records decision references; execution and publication revalidate at specified boundaries.

## Offline and outage behavior

Standalone operation uses local configuration and available inputs. Managed offline operation can use only previously authorized, unexpired capabilities for operations explicitly permitted offline. Tool revocations may not be instantly known while disconnected, so maximum offline windows are a policy choice and an explicit limitation.

Expired authorization, missing required policy, or unverifiable mandatory configuration fails closed. A platform outage must not cause per-request unbounded retries or a public source fallback. Builds that need no new protected access may continue only within their already granted scope; production publishing always requires the appropriate fresh authorization.

## Compatibility and observability

Clients advertise supported schema and enforcement capabilities. The server selects a compatible contract or refuses unsupported mandatory rules. Unknown mandatory constraints cannot be ignored. Requests carry correlation IDs and safe reason codes; credential values and sensitive environment data never enter default telemetry.

Audit emission is bounded and redacted. Failure to upload optional metrics does not fail standalone builds. Mandatory enterprise audit delivery/grace is an explicit policy with a recoverable queue and visible backpressure.

## Acceptance scenarios

- PROTO-01: A standalone build/install succeeds without contacting an account service.
- PROTO-02: Managed logout, entitlement expiry, or platform outage cannot unlock public acquisition.
- PROTO-03: A client missing enforcement capability cannot receive a misleading permissive configuration.
- PROTO-04: Privileged server operations reject client-forged release classifications.
- PROTO-05: Expired or wrong-tenant cached policy cannot authorize a request.
- PROTO-06: Public protocol conformance runs against a mock without private source.

## Open decisions

Finalize wire schemas, auth flows, policy snapshot signing, supported offline windows, protocol support lifecycle, and audit delivery semantics. The contract requires public review before server-specific implementation hardens it.

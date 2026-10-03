---
id: OEP-0010
title: Connector and credential contracts
status: draft
implementation: not-started
updated: 2026-10-01
authors: [micahlmartin]
reviewers: []
requires: [OEP-0002]
tracking-issue: null
---

# Connector and credential contracts

> Design draft for review. MUST and SHOULD express proposed normative requirements, not shipped behavior. Example syntax and protocol fields are provisional unless identified as an agreed product constraint.

## Problem and outcome

Administrators should describe existing systems once, then let policy select them. Public connector contracts allow independently developed clients and adapters without embedding vendor-specific decisions in the build engine.

## Connector descriptor

A connector describes its stable identifier, system/adapter type, approved endpoint, logical repository mappings, supported operations, credential strategy, and capability/version information. Secret material never appears in the descriptor. Example conceptual data:

```json
{
  "id": "corporate-python",
  "kind": "artifactory",
  "endpoint": "https://packages.example.com",
  "repositories": {"python-read": "pypi-approved"},
  "operations": ["metadata.read", "artifact.read"],
  "credentialStrategy": "scoped-token"
}
```

This is an illustrative contract, not a finalized wire schema. Credentials and internal endpoint details are tenant configuration, not public catalog contents.

Policy selects connector plus repository plus allowed operation for tool acquisition, package reads, cache reads/writes, snapshot publishing, release publishing, and signing. Access to one does not imply access to the others.

## Resource authorization boundary

Membership context and permission-check semantics are owned by the [public access-context contract](../OEP-0016-platform-protocol/access-context.md). An active account membership or resource-access allow alone is not a connector-use lease. The connector/operation/purpose restrictions and credential lifecycle below remain separate; this link does not change their mechanics.

## Credential brokerage

The agent authenticates a user/session to the platform. CI uses independently verified workload identity. The platform authorizes a narrow request and uses the connector's supported mechanism to obtain a short-lived, scoped upstream credential. Where direct OIDC federation is possible, it is preferred; OIDC is not assumed to work uniformly across every backend.

A credential lease binds subject, tenant, connector, repository/operation scope, audience, expiry, and refresh eligibility. The client MUST NOT be able to request a broader scope by changing a URL. Upstream-issued restrictions are preferred to restrictions implemented only in the local agent. If a backend supports only a broad static token, the capability limitation is explicit and policy can reject the connector for that use case.

Upstream access tokens live in agent memory for the shortest practical period. Refresh/session material uses OS-protected storage with user-bound access. The platform stores connector secrets in its secret store and audits issuance without logging values. Rotation schedules, maximum lease durations, refresh thresholds, revocation behavior, and break-glass administration are server-controlled.

Registry tokens MUST NOT be exported to shell environments or native package-manager files. Local route credentials, where required, authorize only the local agent and are a separate credential class.

## Data plane and capability boundaries

The client connects directly to the selected approved upstream. The platform issues control decisions and credentials; it is not the default package streaming path. No customer-hosted gateway is part of this design.

A connector that cannot supply safe direct-client access must fail the relevant policy requirement with a capability explanation. Do not quietly fall back to public internet downloads or invent a relay. Supporting such a connector later requires an explicit new proposal.

Adapter code normalizes metadata, pagination, protocol errors, auth refresh, redirects, and repository capabilities. It cannot grant additional authorization. TLS validation is mandatory; a corporate CA can be provisioned through protected configuration. Endpoint changes require revalidation and cannot reuse tokens for a different audience.

Source-control connectors additionally report verified repository/commit identity, event context, protected ref/ruleset facts, and API permission limitations. “Unavailable” or “not authorized to inspect” is not equivalent to “unprotected,” nor sufficient release evidence.

## Threats and operational failures

An administrator of the local machine can ultimately inspect a process or use its privileges. The guarantee is reduced exposure to ordinary developer tooling plus short-lived, scoped, auditable credentials, not impossible extraction from a hostile privileged host.

Clock skew, expired tokens, upstream 401/403, and platform outages have distinct errors. Retry token issuance with bounded backoff; never retry a denied operation using a more privileged credential. Cached descriptors do not authorize use after their permission window expires.

## Acceptance scenarios

- CONN-01: One Artifactory connector supplies separate read, snapshot-write, and release-write scopes.
- CONN-02: Wrong tenant, repository, audience, or operation cannot obtain an expanded lease.
- CONN-03: Short-lived credentials refresh without entering environment/configuration output.
- CONN-04: A backend lacking required scope/expiry support is rejected by policy.
- CONN-05: Package bytes travel workstation-to-upstream without platform streaming.
- CONN-06: Source-control API permission failures remain unknown facts and cannot confer release eligibility.

## Open decisions

Finalize per-backend capability matrices, credential transport protection beyond TLS, minimum native package protocol set, and rotation/revocation semantics for each connector. Adapter integration tests against real supported versions are required before advertising compatibility.

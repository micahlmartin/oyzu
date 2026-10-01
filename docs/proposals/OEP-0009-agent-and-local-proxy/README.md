---
id: OEP-0009
title: Agent lifecycle and local package proxy
status: draft
implementation: not-started
updated: 2026-10-01
authors: [micahlmartin]
reviewers: []
requires: [OEP-0002, OEP-0010]
tracking-issue: null
---

# Agent lifecycle and local package proxy

> Design draft for review. MUST and SHOULD express proposed normative requirements, not shipped behavior. Example syntax and protocol fields are provisional unless identified as an agreed product constraint.

## Problem and outcome

Native package managers should use corporate registries automatically without receiving upstream credentials. A headless local agent provides the stable endpoint, identity, policy routing, and token handling.

## Process and ownership

`oyzu agent run` runs the agent from the same executable as the CLI. Proposed service installation creates a per-user background service by default; a system service requires an explicit multi-user design and privileged administration. Desktop, CLI, and shell clients use the same agent. Closing the desktop does not terminate the service.

The agent owns authentication sessions, protected credential references, policy/catalog snapshots, short-lived tokens, local endpoint configuration, and connection pooling. Build planning and execution belong to the CLI/engine; the agent is not required for an ordinary standalone build that needs none of these managed facilities.

A local control API and registry data endpoints are logically separate. Prefer user-restricted Unix sockets or Windows named pipes for control. Package clients typically need loopback HTTP(S); those endpoints bind loopback only and expose a narrow package protocol, not administrative operations.

## Example: Python installation

1. The CLI reads machine management settings before considering user login.
2. An authenticated agent receives a scoped route descriptor for the approved Python connector.
3. Oyzu configures pip's effective index to the local route using supported native configuration or an Oyzu-managed environment.
4. pip requests package metadata from the agent.
5. The agent checks route scope and current policy, obtains or refreshes a connector credential, and calls Artifactory directly.
6. It rewrites allowed metadata URLs where necessary so package bytes and subsequent requests continue through the agent, then streams the response.

The central platform handles policy and credential control traffic. It does not carry package bytes. If Artifactory is reachable only over a corporate VPN, the workstation must have that connectivity; no customer-hosted gateway is introduced.

## Endpoint security and native configuration

Loopback binding alone is insufficient: another local user or a browser may reach the endpoint. Authenticate data endpoints with a local, narrowly scoped capability where the package manager supports it. That capability may exist in local package configuration, but MUST NOT be an upstream registry credential. Apply restrictive file permissions, host/origin checks, bounded request sizes, and no permissive CORS.

Project-specific access cannot be inferred reliably from an ordinary HTTP request. Route identifiers therefore bind to an explicit workspace/profile and token scope; stronger isolation may require per-session endpoints or OS identity-aware transports. The implementation MUST document residual capability theft risks and must not promise isolation from a privileged local administrator.

Configuration uses native precedence rules and records prior values for reversal. Global configuration is opt-in; environment/session routing is preferred where it works. Existing corporate settings must not be silently overwritten. Child installers, npm scoped registries, Python index URLs, and redirects require adapter-level coverage. Multiple Python indexes are not used as a transparent fallback that permits dependency confusion.

## Routing and lifecycle rules

The proxy allowlists protocol operations and approved upstream paths. It MUST NOT become an arbitrary CONNECT proxy or forward credentials to a redirected host. Range requests, streaming, cancellation, TLS validation, retries, bounded buffers, and upstream backpressure are required.

Agent unavailability yields an actionable error; native client configuration MUST NOT fall back to public sources. Expired authorization cannot be extended by a stale catalog. In-flight operations have defined grace limits; revocation and logout stop new authorized requests.

## Acceptance scenarios

- AGENT-01: A headless workstation performs pip/npm acquisition through an approved direct upstream route.
- AGENT-02: Upstream tokens are absent from child environments, config, logs, and bundle output.
- AGENT-03: Redirects and rewritten metadata cannot bypass routing or leak authorization.
- AGENT-04: Logout preserves managed state and prevents public fallback.
- AGENT-05: Another local user cannot invoke the control API.
- AGENT-06: Agent restart, credential expiry, and interrupted streaming recover without corrupting downloads.

## Open decisions

Choose endpoint authentication per package protocol, local HTTPS certificate handling where required, port allocation, workspace-scoped routing, and service installation mechanisms. These are release-blocking security design questions, not reasons to route bytes through the platform.

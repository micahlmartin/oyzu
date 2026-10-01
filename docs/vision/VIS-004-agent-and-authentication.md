---
id: VIS-004
status: draft
updated: 2026-10-01
---

# Headless agent and authentication

> Vision draft consolidating agreed product direction. No feature is implemented by this document. Unresolved implementation choices are identified explicitly.


The agent is the background mode of the `oyzu` executable. It manages sessions, protected credentials, managed configuration, and local registry proxy endpoints without requiring a desktop window.

## Journeys and responsibilities

The CLI and optional desktop interface connect over protected local IPC. Package managers use loopback HTTP endpoints. The agent obtains authorized connector routes and credential leases, then proxies requests directly to the configured corporate repository.

The agent handles login renewal, policy refresh, connection diagnostics, safe redirects, and request cancellation. It must not inject upstream tokens into pip/npm configuration, shells, subprocess arguments, build environments, or logs.

## Management model

IT distributes protected machine configuration using OS-native mechanisms. It identifies the organization and platform and requires sign-in. It is not a custom device-enrollment or hardware-certificate requirement.

Unmanaged users can operate standalone. Managed users without a valid session must authenticate or use only explicitly permitted offline capabilities. Logout does not erase managed configuration. Broken configuration or an unavailable agent must never select public fallback.

## Credential model

Prefer OIDC federation or an equivalent exchange into short-lived, audience-bound, scoped credentials. Issuer capability must be verified for each connector. Refresh tokens belong in OS-protected storage; access tokens stay in memory where practical.

Some credential material necessarily exists on the workstation when the agent connects directly to an authenticated upstream. We do not claim protection against a sufficiently privileged local adversary. Connectors unable to satisfy an organization's credential restrictions are unsupported in that configuration; no gateway fallback is in scope.

## Process lifecycle

Desktop users may install the agent as a per-user background process. CI may run it for a job. Closing the desktop interface does not stop proxies. One user's requests must not inherit another user's session. Service startup, upgrade, crash recovery, and endpoint stability need explicit cross-platform behavior.

## Success

Normal `pip install` uses the local endpoint and approved upstream without revealing upstream credentials. Headless login errors are actionable. Revocation and policy expiry have bounded effects. Local callers cannot turn a credential-holding proxy into an unrestricted authenticated forwarder.

## Open questions

Local package-client authentication, project scoping on shared endpoints, and Linux credential-store availability need prototypes. Do not invent an unprotected plaintext storage fallback.

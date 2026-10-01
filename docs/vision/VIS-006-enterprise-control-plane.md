---
id: VIS-006
status: draft
updated: 2026-10-01
---

# Enterprise control plane

> Vision draft consolidating agreed product direction. No feature is implemented by this document. Unresolved implementation choices are identified explicitly.


The enterprise platform centrally manages identity, connectors, policy, and evidence while the same public CLI remains useful without it.

## Administrator journey

An administrator configures an Artifactory connector with an endpoint and credential strategy. Policy selects that connector for local Python dependencies, another repository for CI snapshots, and release destinations subject to evidence. Changes propagate to agents without editing project pipelines or distributing raw secrets in project configuration.

Connectors describe capabilities; policy grants their use. Upstream permissions continue to apply. Read access to production packages is distinct from permission to publish production artifacts.

## Responsibilities and boundaries

The API owns organizational membership, RBAC, entitlements, connector instances, versioned policy/configuration, and audit. The UI administers these resources through the API. Tool/version discovery is filtered by allowlists, and mediated acquisition must preserve those restrictions for exact versions and cached content.

Package traffic goes from the agent to configured customer systems. No mandatory SaaS package data plane, customer-hosted gateway, or advance OCI repackaging service is included. Organizations may need upstream repository controls or scoped tokens to enforce restrictions beyond the official client.

Policy is centrally served in managed mode. Standalone users use built-in behavior and explicit local configuration without a policy server. SSO proves identity; paid entitlements are checked by the platform, not inferred from email addresses.

## Trust

Repository-provider plugins supply authenticated facts about refs, commits, protections, and checks. Execution evidence supplies identity and isolation facts. Policy derives eligibility; a developer cannot self-declare production authorization.

An open-source client cannot enforce workstation-wide restrictions against local administrators. High-assurance production services independently verify evidence and deny unauthorized signing or publication.

## Future scope

Hosted CI/CD, deeper fleet metrics, and an Oyzu-managed registry can be added behind these contracts. They must retain the project-first model rather than expose mandatory workflow authoring.

## Success

One centrally managed change updates a required check or registry route without per-repository pipeline edits. Public client contracts are documented openly. Private implementation remains in the platform repository.

## Open questions

Policy evaluator implementation, connector-specific token exchange, server deployment, and subscription packaging require separate decisions.

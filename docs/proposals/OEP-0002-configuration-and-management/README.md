---
id: OEP-0002
title: Cascading configuration profiles and managed settings
status: draft
implementation: in-progress
updated: 2026-10-01
authors: [micahlmartin]
reviewers: []
requires: [OEP-0001]
tracking-issue: null
---

# Cascading configuration, profiles, and managed settings

> Proposed implementation contract. MUST and SHOULD describe requirements, not shipped behavior. This expansion does not mark the proposal accepted or the existing CLI compliant.

## Outcome and scope

Oyzu uses one configuration engine for standalone developers, locally administered machines, and centrally managed organizations. Most projects need no configuration. Explicit configuration adds tools, environment, tasks, preferences, and exceptional build details without becoming a programming language.

Ordinary configuration supplies overridable values. Administrative policy supplies defaults and independent constraints. A profile selects an optional group of values; it never selects trust or authority. Corporate configuration comes from the platform and is cached locally after verification.

The agreed default permits local builds using an explicitly offline-enabled, verified corporate snapshot for up to 24 hours from issuance. Snapshot expiry or administrator restrictions may shorten this window. Publishing, signing, and obtaining new protected credential grants require fresh authorization. No cached setting grants those privileges.

## Contract map

| Document | Responsibility |
| --- | --- |
| [Resolution and implementation](implementation.md) | Locations, precedence, merge, compatibility, CLI, implementation boundaries |
| [Profiles](profiles.md) | Selection, scope, overlay rules, CI distinction |
| [Settings and constraints](settings.md) | Typed registry, initial settings, enforceable operators |
| [Management protocol](managed-policy.md) | Enrollment, JSON transport, verification, cache, offline state machine |
| [Examples and verification](examples.md) | Concrete configurations and acceptance matrix |

These documents replace the earlier OEP-0002 rules that rejected every unknown field, omitted user/machine layers, and proposed managed TOML. Strict build-record contracts in other OEPs are unchanged. Task, tool, agent, execution, and artifact specifications continue to own their domain behavior; this OEP owns configuration resolution.

## Formats and authority

| Input | Format | Purpose |
| --- | --- | --- |
| Machine/user `config.toml` | TOML | Ordinary defaults |
| `oyzu.toml`, `oyzu.local.toml` | TOML | Project settings and personal overrides |
| Root `build.yaml` | YAML | Optional target inventory and build relationships |
| Protected `management.json` | JSON | Organization, platform endpoint, enrollment and verification keys |
| Protected `admin-settings.json` | JSON | Local administrative defaults and constraints, including standalone installations |
| Platform snapshot | Signed JSON payload | Authoritative corporate defaults and constraints |
| `oyzu.lock` | Existing lock contract | Resolved tool identities, not policy or credentials |

A JSON file does not gain authority merely by containing a `locked` field. Protected provenance or authenticated verification establishes authority. HCL is deferred; a future authoring frontend could compile to the same JSON protocol without adding client evaluation.

Default precedence, lowest to highest: built-ins, corporate defaults, local administrative defaults, machine settings, user settings, root-to-target project files, eligible root-to-target local overrides, invocation options. Each layer applies its base values followed by the selected profile. Constraints from all applicable administrative sources intersect after resolution; a higher default layer cannot replace them.

## Context and security boundaries

CI execution context is inferred through detectors. There is no writable `execution.context`, `trusted`, or `releaseAuthority` setting. CI environment hints may change defaults or impose restrictions but cannot grant privileges. A profile called `ci` or `production` is just a name. Independently verified workload identity, source evidence, and backend authorization determine artifact authority.

Management is established before ordinary settings and before login. Logout, unreadable bootstrap, failed refresh, and an expired cache cannot switch an enrolled machine to direct public downloads. Configuration is frozen for each build; later refreshes apply to later plans and fresh privileged authorization.

The client cannot prevent a machine administrator from replacing the client or a user from running unrelated download tools. Network controls, registry authorization, and publication/signing services enforce those boundaries independently. No desktop UI is required.

## Acceptance criteria

- CFG-01: identical inputs produce identical effective values and stable origin explanations.
- CFG-02: invalid enrollment, logout, and expired policy never enable standalone fallback.
- CFG-03: task overrides replace complete definitions while preserving required builder contracts.
- CFG-04: boundaries, nested scopes, and path handling prevent sibling configuration leakage on every supported OS.
- CFG-05: inferred CI reports excluded personal settings; spoofing CI or selecting a profile grants no authority.
- CFG-06: optional unknown fields warn and survive edits; duplicate keys, invalid known values, unsupported major versions and required capabilities fail.
- CFG-07: standalone use needs no account; protected local administration uses the same constraint evaluator.
- CFG-08: exactly one optional profile is selected deterministically without inheritance, expressions or recursion.
- CFG-09: locked values, allowed sets, bounds and mandatory sets compose restrictively and reject conflicting overrides.
- CFG-10: array replacement, atomic task replacement and explicit removal have deterministic semantics.
- CFG-11: policy verification binds identity and scope; refresh is atomic and detects observable rollback and corruption.
- CFG-12: offline local builds obey the signed permission and the shorter of expiry or the at-most-24-hour age limit; privileged operations require fresh authorization.
- CFG-13: configuration is immutable within a plan; privileged actions reauthorize without rewriting recorded evidence.
- CFG-14: inspection explains origins and restrictions without exporting secrets; edits preserve unrelated data and detect concurrent changes.
- CFG-15: unknown mandatory policy semantics block affected operations instead of being silently ignored.
- CFG-16: protected locations and permissions are validated across Windows, Linux and macOS; ordinary overrides cannot unenroll a machine.
- CFG-17: computation identities capture effective build settings and exclude credentials, presentation preferences and incidental host locations.
- CFG-18: native tool constraints, lock identities, registry routing and mandatory reports remain effective regardless of profile or task overrides.

## Delivery, alternatives, and open decisions

Implement the typed source model before loosening current strict deserialization. Follow with profiles and diagnostics, local policy, then verified platform distribution. Keep migrations explicit and reviewable. Existing projects without new sections retain their ordinary syntax. Older binaries cannot be retroactively made compatible: adopting a required feature requires upgrading them first.

Rejected approaches include making administrative policy merely the last override layer, treating login as enrollment, making profiles an authorization mechanism, always rejecting optional unknown fields, and adding a general expression language.

No further product decision is needed to begin implementation against this draft. OS permission fixtures, signed-protocol interoperability, and workload-provider identity tests are release gates rather than claims of completion. The 24-hour offline choice is user-confirmed; field names, transport details and initial limits below are proposed engineering decisions subject to review.

---
id: OEP-0002
title: Configuration and managed settings
status: draft
implementation: not-started
updated: 2026-10-01
authors: [micahlmartin]
reviewers: []
requires: [OEP-0001]
tracking-issue: null
---

# Configuration and managed settings

> Design draft for review. MUST and SHOULD express proposed normative requirements, not shipped behavior. Example syntax and protocol fields are provisional unless identified as an agreed product constraint.


## Problem and agreed constraints

Developers need minimal checked-in configuration and safe personal overrides. Managed installations must never become standalone because login expires, a file is malformed, or the platform is unavailable.

The agreed files are `oyzu.toml` for tools/environment/tasks, ignored `oyzu.local.toml` for local overrides, optional `build.yaml` for targets, and a committed tool lock. This draft proposes the lock name `oyzu.lock`. No programming language, loops, arbitrary evaluation, or shell substitutions are added to configuration parsing.

## Resolution model

Resolve independent default layers in this order: built-in/builder defaults, organization defaults when managed, project hierarchy from root to selected directory, permitted local overrides, then explicit invocation options. Mandatory constraints are evaluated separately after resolution and can deny any resulting value.

Machine management selectors are read before these layers. Proposed mechanisms: HKLM settings on Windows, managed preferences or root-owned settings on macOS, and root-owned `/etc/oyzu/managed.toml` on Linux. Final paths and ACL details require platform validation. There is no custom device-certificate enrollment prerequisite.

The proposed repository boundary is the nearest Git worktree root or explicitly chosen workspace root. Do not climb into unrelated parent directories. Nested TOML augments project-local settings. Only root `build.yaml` is supported initially; nested build files require an explicit future contract.

## Merge and scope rules

Proposed semantics: tables merge by key; scalars and arrays replace; task definitions replace as a unit to avoid inheriting a stale command or environment. No implicit deletion syntax initially. Each effective value carries origin, scope, and policy restriction metadata.

Tools declared in nested scopes select that scope's toolchain without changing other terminals. An unresolved intersection of project runtime constraints and explicit tool versions is an error.

Managed policy determines whether local overrides are permitted in CI. Proposed CI default: ignore developer-local override files and reject attempts to use them for protected publication without explicit policy permission. Record participating layers in the plan without exposing secret values.

## User interface

`oyzu config explain` is a proposed inspection command. It shows the source of each setting and why a requested override was denied. Unknown fields and unsupported schema versions fail with locations and suggestions. Duplicate YAML/TOML keys are errors. Parser limits bound nesting and file size.

Config examples in visions are provisional. Product names and target keys are data, not executable code. Trust authorization is required before running project-provided hooks; parsing literal settings alone does not authorize script execution.

## Failure and trust

Missing management configuration is standalone only when no protected management indicator exists. An unreadable existing management record fails closed. Logout removes session access, not administrative settings. An unavailable platform permits only operations covered by valid cached configuration and explicit offline rules.

Machine-wide controls do not defeat a local administrator modifying the client. Server-side authorization and upstream/network controls remain independent enforcement boundaries.

## Acceptance criteria

- CFG-01: the same layers produce the same effective nonsecret configuration and origin map.
- CFG-02: invalid managed state, logout, and expired configuration never permit public fallback.
- CFG-03: an explicit task override replaces its body while preserving the builder operation contract.
- CFG-04: nested project switching does not leak overrides to sibling projects.
- CFG-05: CI reports exactly which local settings were ignored or accepted.
- CFG-06: duplicate keys and unsupported versions fail before execution.

## Rollout and open decisions

Implement schema validation before activation. Migrate configuration only through an explicit command with a reviewable diff. Open: exact schema version declaration, user-global settings filename, target-local task scope syntax, and opt-in compatibility with mise configuration names.

---
id: OEP-0015
title: Desktop integration and distribution
status: draft
implementation: not-started
updated: 2026-10-01
authors: [micahlmartin]
reviewers: []
requires: [OEP-0009]
tracking-issue: null
---

# Desktop integration and distribution

> Design draft for review. MUST and SHOULD express proposed normative requirements, not shipped behavior. Example syntax and protocol fields are provisional unless identified as an agreed product constraint.

## Problem and outcome

Users should be able to install one cross-platform CLI, optionally enable its background agent, and optionally add a native desktop interface. None of those choices should create a second policy or build implementation.

## Component boundaries

The CLI and agent share Rust libraries and ship in one executable. The optional desktop shell is proposed as Tauri with a TypeScript/React interface. Its backend exposes a small typed interface to the local agent; it does not duplicate token brokerage, routing, task resolution, or build planning.

The desktop displays account and organization state, management source, agent health, tools, approved catalogs, proxy routes, policy explanations, and credential lease metadata without secret values. It can request existing CLI/agent operations with explicit user intent. A normal build needs no desktop process, browser, or UI automation.

The frontend renderer cannot read raw connector credentials or invoke arbitrary operating-system commands. Local IPC authenticates the user and validates every operation. Web content cannot use the desktop as an unrestricted native bridge. Updates to UI assets are part of the signed application distribution, not arbitrary remote pages with native privileges.

## Distribution and updates

Release artifacts identify CLI version, platform/architecture, integrity digest, signature/verification material, and supported agent protocol range. Distribution must cover Windows, Linux, and macOS, with supported architectures explicitly enumerated per release. Availability of one host binary does not imply all build target platforms are supported.

Standalone updates can use approved public release distribution. Managed updates use permitted enterprise routes and policy-selected versions. No background process self-updates from public internet when managed. Enterprise packaging should support system administrators' existing deployment tools.

Updates stage new files atomically and preserve running build/tool versions. Windows executable locking requires a safe replace-on-restart or equivalent flow. Agent/client protocol negotiation prevents an incompatible CLI from corrupting agent state. Rollback must respect security revocations and storage schema compatibility.

OS-specific signing, notarization, package registration, and service integration require separate verified packaging steps. Do not describe downloaded unsigned executables as trusted merely because HTTPS succeeded.

## Installation and removal

CLI-only installation is first-class. Agent service setup and shell-profile integration are independently selectable and reversible. The desktop can attach to an already installed agent or guide supported setup; it must not install a conflicting second agent.

Uninstall removes only owned files and profile blocks. It explains retained tool caches and credentials and offers explicit cleanup choices. Protected managed settings remain administrator-owned; user uninstall/logout cannot clear the enterprise requirement in other managed installations.

Diagnostic export includes component versions, capability checks, endpoint reachability categories, and redacted event IDs. It excludes tokens, full environment dumps, and package contents by default.

## Accessibility and operations

The desktop supports keyboard navigation, accessible names, readable status messages, and OS theme settings. Authentication uses the approved external-browser/device flow as appropriate; headless authentication remains supported.

A crashed or closed desktop must not terminate a build or package download owned by the agent/CLI. A failed agent can be restarted independently, with state recovery limited to valid leases and verified downloads.

## Acceptance scenarios

- DIST-01: CLI-only install, tool use, and build operate on each supported OS without a desktop.
- DIST-02: Closing/restarting the desktop leaves existing agent work intact.
- DIST-03: An incompatible CLI/agent pair fails with a supported recovery path.
- DIST-04: Managed updates cannot bypass the approved distribution route.
- DIST-05: Install/update/uninstall preserve unrelated shell configuration and active tool versions.
- DIST-06: Renderer compromise cannot directly read connector token material through exposed IPC.

## Open decisions

Confirm Tauri after packaging/accessibility prototypes, select installers and architecture matrix, define update/signing infrastructure, and decide minimum backward-compatible protocol windows. Performance and memory budgets require measurement, not invented release claims.

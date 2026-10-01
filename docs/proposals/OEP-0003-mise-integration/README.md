---
id: OEP-0003
title: In-process mise integration
status: draft
implementation: not-started
updated: 2026-10-01
authors: [micahlmartin]
reviewers: []
requires: [OEP-0002]
tracking-issue: null
---

# In-process mise integration

> Design draft for review. MUST and SHOULD express proposed normative requirements, not shipped behavior. Example syntax and protocol fields are provisional unless identified as an agreed product constraint.


## Problem

Version switching, PATH restoration, shell activation, and backend compatibility are difficult to reimplement correctly. Oyzu should reuse mise-derived Rust functionality in-process. Invoking or shipping a separate mise executable is explicitly prohibited.

## Proposed integration boundary

Preserve tool resolution, installation state, activation, shims, and task/environment behavior as a coherent subsystem where feasible. Place an Oyzu-owned facade around these operations. Introduce acquisition and management interfaces without rewriting mature platform behavior unnecessarily.

The investigation must compare a maintained source fork with narrowly vendored internal crates. Do not assume the top-level mise crate is a supported stable embedding API. Pin and record the exact upstream commit before inspecting or copying code; moving-main observations are not a reproducible integration decision.

Proposed facade operations include detect_config, resolve_toolchain, install_distribution, compute_environment_delta, resolve_executable, and enumerate_ecosystem_tasks. These names describe interfaces to investigate, not verified upstream APIs.

## Compatibility contract

Track compatibility separately for tool identifiers/backends, version selection, directory precedence, shell hooks, shims, environment features, lightweight tasks, plugin interfaces, and existing lock/config formats. Each entry has verified/partial/unsupported state and fixtures.

Do not claim drop-in replacement until that matrix is tested. Oyzu-native grouped tasks, build contracts, and managed restrictions take precedence where compatibility would allow a bypass. Report intentional differences.

## Network interception and extensions

Inventory every outbound path: HTTP clients, Git commands, package managers, backend scripts, self-update, and plugin-triggered downloads. Replacing one HTTP client is not complete mediation. Managed paths require approved connectors, downstream token scopes, and sandbox/network enforcement where relevant.

Opaque plugins may require restricted subprocesses for their own implementation, but not a separate mise executable. Unsupported managed behaviors fail explicitly. Standalone compatibility does not certify enterprise suitability.

## Upstream maintenance and licensing

Maintain an upstream commit record, minimal patch series or isolated modifications, attribution, license notices, and a scheduled compatibility review process. Compare behavior and benchmarks before accepting updates. Never auto-merge upstream releases directly into a trusted distribution.

Mise's root MIT license permits reuse subject to preservation of its notice. Dependencies and vendored registries/plugins require independent inspection. Oyzu's own public license remains undecided; no code import is authorized by this draft alone. [Upstream license](https://github.com/jdx/mise/blob/main/LICENSE).

## Verification

- MISE-01: one delivered Oyzu executable switches between two Node projects without a mise executable on PATH.
- MISE-02: departing a directory restores PATH and prior values on supported shells.
- MISE-03: `oyzu exec` works with no shell hook or desktop.
- MISE-04: managed negative tests cover subprocess downloads as well as ordinary HTTP.
- MISE-05: concurrent installs and active versions cannot corrupt one another.
- MISE-06: warm hook and cold activation benchmarks compare against the pinned upstream on equivalent hosts.

Measure p50/p95 hook latency and filesystem/process activity; set regression limits after obtaining baseline data. No unmeasured performance promise is made here.

## Rollout and open decisions

First produce a source dependency map and small integration experiment. Then select fork versus vendoring in a reviewed amendment. Preserve rollback to the preceding Oyzu release. Open: exact source boundary, supported mise formats, plugin sandbox ABI, and update cadence.

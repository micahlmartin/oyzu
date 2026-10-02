---
id: OEP-0003
title: In-process mise integration
status: draft
implementation: in-progress
updated: 2026-10-02
authors: [micahlmartin]
reviewers: []
requires: [OEP-0002]
tracking-issue: null
---

# In-process mise integration

This is the implementation proposal for Oyzu tool management, informed by the completed [experiment](experiment.md) and [qualification](qualification.md). MUST, MUST NOT and SHOULD express proposed requirements. The proposal is implementation-ready for review; it is not accepted, and the production CLI does not yet implement it. Experiment results do not satisfy production acceptance gates. The maintainer authorized staged implementation, including tool management, on 2026-10-02. Individual acceptance and release gates remain in force.

## Decision and intended outcome

Use a pinned, maintained source fork of the mise Rust library behind a private Oyzu facade. Reuse upstream version interpretation, backend metadata knowledge, supported installation/layout behavior, environment calculation and shell rendering. Oyzu owns configuration, lock identity, authorization, committed store records, acquisition transport and process supervision. Do not extract selected internal mise crates into a second independently maintained tool manager.

One delivered `oyzu` executable links the library. It may start another instance of **that same executable** as an isolated internal worker. It MUST NOT build, ship, locate or invoke a separate mise executable. This preserves in-process library reuse within each worker while containing upstream process-global state. The CLI and headless agent remain usable without a desktop. Standalone use needs no account or private repository.

Project authors continue to write ordinary Oyzu TOML:

```toml
[tools]
node = "22"
python = "3.13"

[env]
APP_MODE = "development"
```

Versions illustrate syntax, not supported-version recommendations. `oyzu.lock` records exact selections. No generated `mise.toml`, `.mise.toml`, `.tool-versions` or `mise.lock` participates in selection. Those files may coexist but remain inert to Oyzu; explicit future import would be a separate editing command. Ambient `MISE_*`/`__MISE_*`, home mise configuration and mise caches MUST NOT override Oyzu. User configuration and administrative policy come through OEP-0002, never a second resolver.

## Normative document map

These companions are parts of this OEP, not optional implementation notes:

| Document | Contract |
| --- | --- |
| [Runtime and commands](runtime.md) | Module ownership, library facade, worker protocol, CLI, environment, shims and cancellation |
| [Lock and installation store](lock-and-store.md) | Format 2, scoped selections, dependency identity, receipts, transactions, recovery and leases |
| [Acquisition and authorization](acquisition.md) | Backend admission, broker protocols, corporate policy, native host installs and fail-closed behavior |
| [Upstream maintenance](upstream-maintenance.md) | Fork ownership, update cadence, patch tracking, promotion gates, security fixes and rollback |
| [Implementation and acceptance](implementation.md) | Ordered work packages, tests, platform matrix, performance gates and release criteria |
| [Experiment](experiment.md) / [qualification](qualification.md) | Historical evidence; does not override normative contracts |

OEP-0002 owns configuration scope and management protection; OEP-0004 owns the general acquisition lifecycle; OEP-0008 owns shell behavior; OEP-0009/0010/0016 own the agent/connector/policy boundary. This OEP concretizes their tool integration contracts. Its proposed **format 2** supersedes the experimental format-1 lock and earlier OEP-0004 format-1 sketch. It does not change native package-manager locks, `build.yaml`, or build-record schemas. Related proposals link this refinement; cross-references do not add circular metadata dependencies.

## Why this boundary

The pinned unmodified library cannot construct the required discovery-free configuration through a public entrypoint. The experiment required four patched files: empty configuration construction, frontend shim identity, HTTP mediation and verbatim PATH joining. It also found global initialization/settings assumptions. A reviewed fork with a narrow embedding module makes these dependencies explicit.

Real Node, Go, Java, Python, Aqua/jq and npm/Prettier worked through Oyzu's public broker in a Linux worker. Bash/Zsh lifecycle passed on Linux and native macOS. Windows exposed cancellation and `.cmd` argument failures. Cached execution accepted a changed lock digest, and an asdf plugin's hard-coded curl could not use HTTP URL replacement.

| Finding | Required implementation response |
| --- | --- |
| Upstream version directory is not a distribution receipt | Require an Oyzu receipt, tree identity and current authorization; a mise installed-version check is insufficient |
| Global state can cross workspace boundaries | One immutable context per short-lived same-binary worker; no concurrent project contexts in agent mise globals |
| Native clients bypass library HTTP | Broker-only worker networking plus qualified native protocol adapters; scripts default to denied |
| Windows command wrappers lose characters | Native executable shim and typed launch descriptor; no generic command-shell round trip |
| Windows frontend termination orphans children | Job Object supervisor, kill-on-close and suspended child assignment before execution |
| Prompt hooks cannot refresh the network | Local verified selection cache; agent refresh occurs separately; stale authority produces an unavailable state |

## Scope and compatibility

The scope includes selection/switching, exact locking, installation, activation, direct execution, shims, status, explicit update and safe pruning on Windows, macOS and Linux. Development environments and hermetic build tools share identities, not ambient PATH or shell state. Builders retain native project/task behavior. Mise tasks, configuration templates, arbitrary plugin settings and directory-entry installation are not adopted.

Initial admission is finite: core prebuilt Node, Go, Temurin Java, prebuilt Python and the pinned Aqua jq definition. Native npm package tools follow behind their dependency-closure gate. Admission is by backend, distribution and target platform, not backend name alone. A listed backend may reject a particular version/platform/layout. The [acquisition matrix](acquisition.md) defines the gates.

Rust/rustup, additional Aqua tools, other package managers, asdf/vfox plugins, source-built tools and arbitrary installation scripts remain disabled until their admission work passes the same gates. This does not remove agreed builder ecosystems: provisioned images remain usable, and a builder's implementation does not imply a supported installer.

Managed native prebuilt tools use an explicit artifact/layout plan and trusted host finalization. Foreign executables never run in a Linux worker. A backend unable to separate target-independent acquisition from host execution is not admitted to this path. Scripted/source-built installation requires a separately qualified executor profile; it never falls back to an online host process.

## Source, build and licensing contract

Use the public `oyzuai/mise` fork through a Rust Git dependency pinned to a full reviewed commit, with an explicit feature set and Oyzu's root Cargo.lock. The [upstream maintenance procedure](upstream-maintenance.md) defines source provenance, the patch register, initial-base qualification and the two-repository promotion process. The tested upstream base is `da0db43e9398b46bafa95232014708a51120e731` (mise package 2026.9.18); the fork's later creation revision is not automatically qualified. Build and package the library target only. Source integration is a later work package, not performed by this document. A vendored snapshot is optional future packaging, not a second source of maintained changes.

The release manifest identifies source pin, patches, dependency lock, enabled features, compiler/target, bundled registry snapshot, adapter ABI and backend compatibility digest. No runtime pull of upstream main, registry main or plugin head is allowed. Initially qualify the tested rustls/vendored-Lua feature combination; disable unused plugin execution regardless of compiled features. Removing unused features changes the qualified binary and requires affected suites to rerun.

Before import/distribution, preserve the MIT notice attributed to Jeff Dickey, audit shipped dependencies/registry data/plugins, produce notices and an SBOM, and record maintainer approval of license compatibility. The experiment inventory is not that approval. This OEP does not choose Oyzu's public license. An unresolved license decision blocks import/release, not public documentation, fixtures or owned interfaces.

On 2026-10-02 the maintainer explicitly authorized controlled development import
of fork revision `9290bcac695c8ff8a56760ccebd785d5062b459c`, with defaults disabled
and `rustls` plus `vendored-lua` enabled. This permits the opt-in functional proof;
it does not approve distribution or complete the shipping license audit. The
maintainer also reserved hardening for future goals: the current implementation
goal covers functional behavior, usability and user acceptance. Keep deferred
hardening visible without treating it as a prerequisite for this development work.

Check upstream weekly and triage relevant security advisories on the response targets in the [maintenance procedure](upstream-maintenance.md). Prepare routine promotions monthly. Every update requires a reviewed fork PR and an Oyzu pin/lock/provenance PR, with qualification against the final exact commit. Never auto-merge or silently rebind a lock to a changed backend. Historical backend compatibility requires an explicit reviewed compatibility entry; otherwise require explicit relocking with a visible diff.

## Acceptance criteria

Original IDs are retained and made concrete in the [acceptance matrix](implementation.md). Additional IDs cover experiment failures.

- MISE-01: One delivered Oyzu executable switches two real locked Node projects with no mise executable available; Oyzu files alone control selection.
- MISE-02: Enter/switch/leave restores owned environment changes while preserving duplicate PATH entries and unrelated user edits on every admitted shell.
- MISE-03: Headless execution and shims work without activation or desktop, with exact argument, working-directory, status and cancellation contracts.
- MISE-04: Real broker/executor tests deny HTTP, redirect, DNS, direct-IP, proxy, native-client and child-script bypasses without exposing upstream credentials.
- MISE-05: Concurrent installs, interrupted commits, running-version leases and prune/recovery cannot expose incomplete or mismatched installations.
- MISE-06: Paired in-process upstream/facade benchmarks and end-to-end hooks meet the regression gate on equivalent pinned hosts.
- MISE-07: Cached selection rejects changed distribution/backend/dependency/tree identities even when upstream reports the version installed.
- MISE-08: Canonical lock parsing, scoped multi-version selection, cross-platform records and frozen commands obey format-2 rules and reject ambiguous closures.
- MISE-09: Separate worker contexts cannot inherit mise overrides, secrets, another project's settings or a prior task's environment.
- MISE-10: Native Windows launch preserves edge-case arguments and killing the supervisor terminates its entire managed child tree.
- MISE-11: Managed cached selection, logout, expiry, revocation and unavailable policy fail closed under the exact grant contract.
- MISE-12: Every admitted backend/platform has real artifact, verification, dependency, entrypoint and outbound-path evidence; unsupported tuples fail before installation.
- MISE-13: Native host finalization accepts only a verified finite layout plan and rejects malicious archives, links, collisions and unapproved post-install execution.
- MISE-14: Build preflight captures tool receipt identities and never substitutes the activated shell or changes a source lock.
- MISE-15: Import/update/rollback preserves notices, reproducible source identity, lock compatibility and tested behavior without packaging mise's executable.
- MISE-16: Profile editing, shell failure recovery, diagnostic redaction and JSON output pass the specified tests.

## Open decisions and review checklist

Commands, format, storage, process topology and backend gates below are the proposed defaults, not choices delegated to an implementer. Maintainer review must accept or amend them before work is described as implementing an accepted OEP. Review format-2 migration, worker topology, managed native layout admission, grant validity, shell matrix and performance thresholds explicitly.

Public licensing approval and real platform/connector conformance are external prerequisites with named gates, not assumptions. Arbitrary plugins, source builds, persistent offline grants across agent restarts and additional shells require follow-up amendments. No new product constraint is marked agreed solely because it appears here.

---
id: OEP-0008
title: Environment activation and shell integration
status: draft
implementation: not-started
updated: 2026-10-01
authors: [micahlmartin]
reviewers: []
requires: [OEP-0002, OEP-0003]
tracking-issue: null
---

# Environment activation and shell integration

> Design draft for review. MUST and SHOULD express proposed normative requirements, not shipped behavior. Example syntax and protocol fields are provisional unless identified as an agreed product constraint.

## Problem and outcome

Developers need automatic project environments and fast tool switching without hand-written shell logic. The same configuration must also support explicit, headless command execution.

## Configuration and activation

`oyzu.toml` provides tools, an `[env]` table, and tasks; ignored `oyzu.local.toml` provides developer overrides:

```toml
[tools]
node = "22"
python = "3.13"

[env]
APP_MODE = "development"
```

These versions illustrate syntax and are not a recommendation or supported-version promise. Resolution is recorded in `oyzu.lock` under OEP-0004. Environment values and task definitions follow OEP-0002 precedence. Secret literals SHOULD NOT be committed; managed secret references resolve through the agent without persisting tokens in shell setup files.

Shell activation installs a small hook that detects directory/configuration changes and requests an environment delta. Proposed `oyzu exec -- <command>` provides the same resolved development environment without modifying a user's profile. Exact activation commands and supported shell versions are pinned during the mise integration audit.

Automatic activation MUST NOT run arbitrary repository tasks or install tools merely because a developer changes directories. Missing tools produce a clear status or an explicit install action. Repository trust approval controls executable configuration and secret resolution. A trust grant binds to the relevant files and executable behavior; material changes invalidate the grant.

## Switching correctness

On entry, Oyzu captures the prior values it changes and emits only necessary deltas. On exit or switching projects, it restores those values without deleting unrelated user edits. It removes only PATH entries it owns, deduplicates consistently, and handles nested project roots. Shell instances maintain independent activation state.

Tool selection uses the nearest applicable project configuration within the chosen root, then user defaults. Local overrides change development selection but cannot relax mandatory organizational constraints. Frozen build resolution uses the build's recorded configuration, not whichever project a shell previously activated.

Two supported mechanisms may coexist: PATH activation for fast native invocation and shims for resolution when shell activation is absent. Neither mechanism launches a separate mise executable. Shims MUST resolve symlinks, executable extensions, arguments, exit codes, signals, and Windows command dispatch correctly.

## Cross-platform behavior

The initial validation matrix covers Windows PowerShell, macOS/Linux zsh and bash, plus fish if its inherited implementation is supportable. Additional shells require explicit conformance coverage. Windows PATH casing, PATHEXT, paths with spaces, UNC paths, executable locking, and process cancellation require dedicated cases. Unix tests cover symlinked working directories, executable bits, signals, and login/non-login shells.

Profile changes are opt-in, idempotent, bounded to an identifiable block, and removable. Installation cannot overwrite an existing profile. Terminal prompts must not make a network request for every redraw. Cached resolution is invalidated on configuration or lock changes, with benchmarks before performance commitments.

## Environment versus build inputs

The activated development environment is intentionally convenient. A build uses only the explicit action environment admitted by its builder and policy. An unrelated shell variable, local PATH entry, or globally installed compiler cannot accidentally enter the hermetic build.

Secret environment injection for an explicitly requested development command is a distinct feature from registry token brokerage. It inherently exposes that secret to the child process; documentation MUST state this instead of claiming all secrets are invisible everywhere. Registry credentials remain agent-held.

## Acceptance scenarios

- ENV-01: Moving between two projects selects their locked tools and restores the previous environment on exit.
- ENV-02: A directory with executable configuration cannot execute it before trust approval.
- ENV-03: Two terminals in different projects do not affect one another.
- ENV-04: Spaces, Unicode paths, Windows extensions, and command exit codes work across supported hosts.
- ENV-05: Profile installation/removal preserves unrelated user configuration.
- ENV-06: Build input identity does not change when an unrelated shell variable changes.

## Open decisions

Confirm initial shell list, compatible mise syntax, supported secret reference syntax, profile installer UX, and quantitative activation benchmarks. Avoid implementing a second expression language for environment interpolation.

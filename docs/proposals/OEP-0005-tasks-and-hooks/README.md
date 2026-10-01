---
id: OEP-0005
title: Task discovery overrides and hooks
status: draft
implementation: not-started
updated: 2026-10-01
authors: [micahlmartin]
reviewers: []
requires: [OEP-0002]
tracking-issue: null
---

# Task discovery overrides and hooks

> Design draft for review. MUST and SHOULD express proposed normative requirements, not shipped behavior. Example syntax and protocol fields are provisional unless identified as an agreed product constraint.


Implementation detail: [v1alpha1 implementation contract](implementation.md). This companion resolves the initial engineering defaults below; it remains draft, and does not imply maintainer acceptance or verified platform support. Where an older paragraph leaves an implementation choice open, the companion is the proposed initial resolution.

## Problem and agreed behavior

Tasks are the unified developer interface across all builders. They are inferred by builders, discovered from native ecosystems, or declared in TOML. Targets form groups: `oyzu run api:test`. `oyzu run list` lists tasks grouped by target with their origins.

Defining the same qualified task in TOML replaces its implementation. Every primary task has implicit pre/post slots; defining `api:pre_test` or `api:post_test` attaches them. Direct runs and builds MUST use the same executor.

## Example

```toml
[tasks."api:test"]
run = "./scripts/test-api"

[tasks."api:pre_test"]
run = "./scripts/prepare-test-data"

[tasks."api:post_test"]
run = "./scripts/summarize-tests"
```

The sequence is pre_test → test → post_test. The explicit main body still participates in report collection and validation.

## Discovery and names

Proposed identifiers use `target:task` with one reserved separator; explicit root tasks are unqualified. Builder defaults yield to native ecosystem task implementations where the adapter identifies the same intent; explicit TOML wins last. Conflicting discoveries without an adapter rule are errors, not lexicographic choices.

Proposed unqualified resolution: exact root task, otherwise the uniquely selected current target. From a multi-target root, ambiguous names fail and print qualified alternatives. `oyzu run list` is reserved; `oyzu run -- list` explicitly invokes a root task named list. `oyzu run` alone lists tasks. These disambiguation rules remain draft syntax.

List output includes task kind, provider, override origin, hooks, and dependency summary. Hidden main-task overrides remain visible in explain output but cannot be invoked through an undocumented bypass.

## Hook execution

A failing pre-hook prevents the main task and post-hook. A main failure skips post. A post failure fails the invocation. Cancellation propagates to the current process tree; no new hook starts after cancellation. Post is success-only, not finally/cleanup. Engine-managed temporary-resource cleanup always runs separately; custom always-run hooks are not in the initial contract.

Hooks do not recursively gain hooks. Directly invoking a hook executes it once. Dependency cycles involving hooks are rejected. Hook existence is resolved and frozen in the plan so a mid-build config edit cannot inject new work.

Dependency prerequisites complete before the pre-hook. Proposed argument behavior: forwarded CLI arguments go to the primary task only; hook-specific options come from their own definition. Working directories and toolchains derive from the owning target.

## Contracts and safety

An override replaces implementation, not mandatory engine guards or policy checks. Preflight authorization/integrity gates are engine operations and cannot be disabled by a task named preflight. Required tests must still produce acceptable evidence. A custom command needing extra inputs declares them through the task/builder contract.

Discovery MUST NOT execute arbitrary scripts just to list tasks. Native executable discovery uses an explicitly authorized isolated adapter path. Builds choose a dependency graph, not all discovered tasks. `format` edits sources only in an explicit development invocation; build formatting checks are read-only.

## Caching and effects

The task definition model needs only a command, optional working directory/environment, prerequisites, and input/output declarations when inference cannot establish them. Field syntax beyond the shown `run` form is deferred to examples. Dependencies are graph edges, not imperative conditionals or loops. Long-running development servers and interactive tasks are explicitly invoked and do not automatically enter a finite build graph.

New custom commands and hooks are noncacheable by default until their inputs, outputs, and effects are understood. Cacheable hooks are separate graph actions; a main action key includes actual hook-produced input digests and the resolved definition. Noncacheable side-effect hooks still run around a cached main action. Policy can force re-execution of evidence-producing actions.

The engine does not assume an arbitrary hook is pure because its name starts with pre_. Unchanged output content can permit downstream reuse after the hook executes. See OEP-0011 for trust requirements.

## Acceptance criteria

- TASK-01: Go defaults and npm scripts appear without TOML task duplication.
- TASK-02: api:test executes identical hook ordering directly and in a build.
- TASK-03: all three failure positions yield distinct recorded outcomes.
- TASK-04: an override cannot remove mandatory reports or authorization.
- TASK-05: changed hooks affect execution and cache decisions; uncached effects never disappear on a hit.
- TASK-06: list/name ambiguity, cycles, cancellation, and monorepo scope have deterministic tests.

## Open decisions

Task dependency syntax, portable argv versus shell-string support, shell selection on each OS, invocation of overridden originals, and explicit cleanup hooks require review. Existing native tools may have their own hooks; adapters must prevent accidental double invocation rather than disable native behavior silently.

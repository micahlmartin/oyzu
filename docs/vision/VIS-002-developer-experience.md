---
id: VIS-002
status: draft
updated: 2026-10-01
---

# Tools environments and tasks

> Vision draft consolidating agreed product direction. No feature is implemented by this document. Unresolved implementation choices are identified explicitly.


The developer experience should feel like one project-aware tool. Existing package manifests and scripts remain useful inputs rather than being translated by hand into Oyzu configuration.

## Journeys

A developer runs `oyzu install` to resolve the project toolchain and write its lock, enters a project to activate its environment, and runs `oyzu run api:test` to invoke an inferred operation. `oyzu exec -- <command>` provides the same project selection without interactive shell hooks.

Illustrative TOML:

```toml
[tools]
node = "22"
python = "3.13"

[env]
APP_ENV = "development"

[tasks."api:pre_test"]
run = "./scripts/prepare-tests"
```

This is example design syntax, not a released parser contract.

## Responsibilities

Tool resolution, installation, switching, shims, shell integration, directory inheritance, and task invocation belong to this experience. Project activation must restore previous environment values on exit and avoid duplicate PATH entries. Different terminals may have different active projects without changing a shared global selection.

Tasks come from builders, existing ecosystem scripts/targets, and explicit TOML definitions. Listings expose origin and group. Defining the same `target:task` overrides its implementation; `pre_<task>` and `post_<task>` attach hooks. Both direct invocation and builds use the same executor.

## Constraints

Standalone use has no required login. Managed-machine configuration persists independently of a login session and cannot be overridden by project settings. Tool locks preserve exact distributions; permitted updates are explicit.

Task discovery is not permission to execute arbitrary repository code. Activating a development environment and supplying a hermetic build environment are different operations. Upstream registry credentials never become project environment variables.

Mise-derived activation code should be preserved and tested as a coherent subsystem. Compatibility is a documented matrix, not a claim that every mise behavior already works.

## Success and examples

Verify installation and locking, switching between projects, nested scopes, local overrides, environment restoration, headless execution, conflicting versions, and managed refusal. Exercise supported shells on real hosts. Measure warm prompt-hook overhead as well as cold activation; numerical performance budgets require an initial baseline.

## Boundaries and questions

The CLI cannot constrain an administrator running unrelated software. Tasks that edit files or keep servers alive are not automatically hermetic build actions. Detailed precedence and reserved task names are proposed in OEP-0002 and OEP-0005.

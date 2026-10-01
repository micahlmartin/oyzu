---
id: VIS-001
status: draft
updated: 2026-10-01
---

# Oyzu platform

> Vision draft consolidating agreed product direction. No feature is implemented by this document. Unresolved implementation choices are identified explicitly.


Oyzu is an **opinionated, batteries-included developer platform**. It unifies tool installation, environment activation, lightweight tasks, and convention-driven builds in one CLI. An optional enterprise platform supplies organization-wide identity, configuration, policy, and visibility. Humans, automation, and AI agents use the same commands; normal operation is deterministic and does not require an LLM.

## What batteries included means

The product should own the routine integration work between a project's tools, environment, tasks, tests, reports, build outputs, and cache. A developer should be able to move from a checkout to an inspectable result through a coherent workflow, without assembling and maintaining that wiring for each repository.

Opinionated means choosing documented defaults from ecosystem conventions, explaining those choices, and offering small, explicit overrides where a project needs them. Existing native manifests and lockfiles remain inputs to that experience. Minimal configuration must still produce useful testing, reporting, artifacts, and evidence.

Batteries included describes the intended integrated experience, not a promise that every tool is bundled or every capability is available today. The current CLI requires provisioned native tools; tool installation and the wider platform remain incremental delivery work. The [implementation status](../implementation-status.md) is the source for measured capabilities.

## Product contract

A conventional supported project should work with `oyzu build` without an Oyzu build file. When discovery needs help, the smallest declaration is:

```yaml
myapp:
  uses: python/app
```

A **builder** is an implementation such as `python/app`. A **target** is a named instance such as `myapp`. A **task** is an operation such as `myapp:test`. "Recipe" is not the product term.

Project authors describe intent and exceptions. They do not maintain jobs, runners, pipeline stages, cache restoration scripts, reporting plumbing, or organization-specific scanner integrations. Rich internal graphs do not justify rich mandatory configuration.

## Agreed principles

- The standalone CLI works without an Oyzu account.
- One executable contains CLI and headless-agent modes; no separate mise executable is invoked or bundled.
- Windows, macOS, and Linux are first-class host platforms.
- The desktop interface is optional and is never required for builds.
- `oyzu.toml` owns tools, environment, and tasks; optional `build.yaml` owns targets and build exceptions.
- Builders infer tasks, dependencies, artifact types, testing and reporting behavior.
- Build inputs are captured and acquired before isolated execution. No silent network or host-access fallback.
- A build produces a portable `dist/` bundle with a manifest, including on partial failure where persistence is possible.
- Trust is evaluated from authenticated facts and evidence, not branch names, CI environment variables, or a "trusted builder" label.
- Local-origin builds cannot be promoted into production under the agreed managed baseline.
- Standalone remote caching requires only a compatible OCI registry, not a custom cache service.
- Enterprise connectors integrate existing infrastructure, including Artifactory and Nexus.
- Package bytes travel between local agents and approved upstream systems; mandatory SaaS package proxying and a customer-hosted gateway are out of current scope.

## Scope

Initial ecosystem coverage includes Python (multiple packaging approaches), Go, Node.js, Rust, Docker, Helm, and Java through Maven, Gradle, and Ant. Multi-module, workspace, monorepo, and mixed-language projects are part of this scope. Implementation can proceed incrementally without claiming unimplemented coverage.

Later platform capabilities include hosted CI/CD execution, deeper fleet intelligence, and an optional Oyzu-managed registry. Their existence must not be implied by an initial CLI release.

## System boundary

The public repository contains the CLI, agent, builders, connectors, optional desktop UI, and public contracts. The private platform repository owns organizational administration and hosted enforcement. The public repository cannot require private dependencies or credentials.

The recommended implementation stack is Rust for core/services and TypeScript/React for interfaces, with Tauri for the desktop shell. This is an architectural direction, not a completed dependency audit.

## Success

A new conventional project requires zero Oyzu build configuration; a necessary exception is small and explainable. A developer can inspect the plan, setting origins, cache decisions, and evidence. No hidden adoption step requires GUI availability, public network fallback, or a paid account.

## Exclusions

No new programming language in YAML/TOML. No pipeline authoring product. No blanket promise to sandbox every arbitrary tool on every host without validating the execution backend. No advance repackaging of the world's tools into OCI.

## Open decisions

Public license, exact mise integration boundary, minimum OS releases, execution-isolation implementations, and detailed wire schemas remain under review. See the [decision register](../decisions.md).

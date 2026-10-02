# Tasks in captured builds

`oyzu build` composes the selected builders' stages with discovered tasks, explicit replacements, prerequisites and hooks. It captures source and dependencies before executing the resulting graph in private target workspaces. Provision the CLI's native toolchain images and supported Docker executor first; see [image provisioning](README.md) and [acceptance checks](build-verification.md). This reference describes the current experimental executor, not native Windows/macOS target compilation.

## Defaults and customization

Ordinary projects need no task configuration. Builders supply applicable build, test, lint and read-only format-check operations. `oyzu run list` shows discovered tasks without running them. Mutating `format` tasks are for explicit development use and are rejected if included in a captured build. Explicit replacements retain the builder's mandatory report and policy obligations.

A qualified prerequisite uses the existing TOML task syntax:

```toml
[tasks."api:pre_test"]
argv = ["node", "scripts/check-api-inputs.mjs"]
depends_on = ["schema:verify"]

[tasks."schema:verify"]
argv = ["node", "scripts/verify-schema.mjs"]
```

Here `api` and `schema` are targets discovered or declared in `build.yaml`; their scripts are relative to their respective target roots. The example requires Node-compatible toolchains for both tasks. A custom command does not implicitly provision a new tool. Use the native ecosystem's command where appropriate; no new build-file task language is introduced.

The `schema:verify` operation runs once per invocation even if several selected tasks require it. Its working directory, toolchain, resolved configuration, environment constraints and output/report paths belong to `schema`. Its dependencies and `schema:pre_verify` / `schema:post_verify` hooks run under the same ownership rules. `api:pre_test` waits for the prerequisite's post-hook and any deferred report validation before `api:test` can start. Hook names do not recursively acquire their own hooks.

Builder stage order remains in force. A stage already selected as an earlier task's prerequisite is not executed again. Newly selected local prerequisites follow the previous local stage and retain their local dependency-list execution order, including sibling prerequisites. This ordering remains enforced while an earlier stage waits on foreign work; a later helper cannot run early simply because the target's worker is idle. Another target's prerequisites follow their own pipeline. Cycles involving task prerequisites, hooks, builder stage order, target dependencies or materialized artifacts fail before action execution. Error diagnostics currently identify an action cycle without providing its shortest path. Fix the ordering relationship and rebuild; rearranging YAML entries does not resolve a cycle.

An explicit unqualified root task still takes precedence for a single-target build. A root helper reached through a qualified prerequisite inherits that prerequisite's target/toolchain context while retaining its declared root-relative working directory. In multiple-target builds, a root task requested from more than one owner is rejected as ambiguous; qualify a shared task with its actual target. This prevents the first target visited from silently choosing its toolchain or private workspace. Task names and command bodies otherwise keep their existing configuration precedence.

## Isolation, evidence and failure

Task prerequisites are ordering relationships. They do not transfer generated files between private target workspaces. Use declared artifacts and `materialize` when a consumer needs producer bytes; see [directory artifacts](directory-artifacts.md). A prerequisite cannot read another target's mutated execution tree just because its task completed.

Each action records its owner, tool identity, dependencies, environment and report obligations in `dist/plan.json`. The manifest records actual action results, retained reports and versioned artifacts. Packaging waits for every selected task owned by that target, including custom tasks required by another target. A failed prerequisite, post-hook or required report blocks dependent actions and packaging; available evidence remains in the failure bundle. Independent targets may still finish successfully. `oyzu inspect dist` verifies recorded content integrity, not release eligibility.

`build.jobs` bounds ready actions, and the executor never runs two actions concurrently in one target's mutable workspace. Independent targets can overlap. Execution uses captured offline inputs; task dependencies do not grant network, credential or host access. Development `oyzu run` uses the host environment and does not yet produce a standalone captured report bundle.

## Verification and limits

Rust integration checks cover cross-target ownership, required report paths, post-hook collection boundaries, local sibling order and cycles visible only after combining build stages. The `core` captured-build suite exercises a shared custom prerequisite with two consumers, private workspace effects, successful snapshot packages and failure propagation from the producer post-hook. It also tests local prerequisites while compilation waits on a foreign task, and a Node consumer requesting a Go-owned task through the materialization scenario. Its checks execute the compiled CLI with real native tools; suite registration alone is not acceptance. See [implementation status](../implementation-status.md) for revision-specific local and CI evidence.

This adds qualified task composition to the existing builders. It does not complete generated-output inference, task caching, matrix expansion, interactive services or tool installation. Use the authored scenarios and their acceptance criteria to track those remaining capabilities.

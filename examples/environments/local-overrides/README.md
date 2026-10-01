# EX-003: Checked-in defaults and local overrides

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- APP_MODE defaults to development; an explicitly copied local override selects sandbox.
- CI records ignored local overrides and retains checked-in configuration.

## Review the project

Open the checked-in project roots: `project`. Source, native manifests, and Oyzu configuration are included here so we can review the intended experience directly.

The intended Oyzu interface is:

```text
oyzu config explain
oyzu exec -- node probe.mjs
```

To try the override, copy `local-override.toml.example` to `project/oyzu.local.toml` in the disposable copy. The fixture does not commit a real local override.

No build YAML is required for this scenario. Configuration, where present, demonstrates only the feature under discussion.

## Native checks available now

This is an interaction/evidence contract. Review its input files and expected outcomes; no substitute Oyzu implementation is supplied.

Native commands document the underlying ecosystem workflow. They are supporting context, not a prerequisite for accepting or revising this design contract.

## Failure and variation cases

- **local-override:** Copy local-override.toml.example to project/oyzu.local.toml in a disposable copy. Expected: Only local scope changes; the local file stays ignored by Git.

## Contract and limitations

Acceptance criteria: CFG-01, CFG-05. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

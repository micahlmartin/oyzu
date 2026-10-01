# EX-020: Native npm workspace dependency graph

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- The app consumes a shared package through workspace metadata.
- Changing shared code affects the app; changing app code does not rebuild unrelated targets.

## Review the project

Open the checked-in project roots: `project`. Source, native manifests, and Oyzu configuration are included here so we can review the intended experience directly.

The intended Oyzu interface is:

```text
oyzu build
```


No build YAML is required for this scenario. Configuration, where present, demonstrates only the feature under discussion.

## Native checks available now

With the named toolchains already installed, run:

```text
# From project
npm install --offline --ignore-scripts --no-audit --no-fund

# From project
npm test --workspaces
```

Native commands document the underlying ecosystem workflow. They are supporting context, not a prerequisite for accepting or revising this design contract.

## Failure and variation cases

- **duplicate-execution:** Select both workspace root and package targets. Expected: Do not run the same native workspace task twice.

## Contract and limitations

Acceptance criteria: BUILDER-02, PLAN-05. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

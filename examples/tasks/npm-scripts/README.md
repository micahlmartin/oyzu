# EX-007: Native npm scripts and their lifecycle hooks

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- Expose foo, test, and build from package.json.
- npm prefoo/postfoo execute exactly once in native order.

## Review the project

Open the checked-in project roots: `project`. Source, native manifests, and Oyzu configuration are included here so we can review the intended experience directly.

The intended Oyzu interface is:

```text
oyzu run list
oyzu run foo
```


No build YAML is required for this scenario. Configuration, where present, demonstrates only the feature under discussion.

## Native checks available now

With the named toolchains already installed, run:

```text
# From project
node --test

# From project
npm run foo
```

Native commands document the underlying ecosystem workflow. They are supporting context, not a prerequisite for accepting or revising this design contract.

## Failure and variation cases

- **native-hooks:** Run foo through Oyzu. Expected: stdout contains prefoo, foo, postfoo exactly once each.

## Contract and limitations

Acceptance criteria: TASK-01, BUILDER-03. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

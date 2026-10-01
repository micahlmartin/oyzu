# EX-004: Execute without shell hooks or a desktop

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- The configured tool/env resolve with no profile changes or UI.
- Arguments, Unicode paths, and exit code 7 survive execution.

## Review the project

Open the checked-in project roots: `project with spaces`. Source, native manifests, and Oyzu configuration are included here so we can review the intended experience directly.

The intended Oyzu interface is:

```text
oyzu exec -- node probe.mjs
oyzu exec -- node probe.mjs --fail
```


No build YAML is required for this scenario. Configuration, where present, demonstrates only the feature under discussion.

## Native checks available now

With the named toolchains already installed, run:

```text
# From project with spaces
node probe.mjs

# From project with spaces
node probe.mjs --fail
```

Native commands document the underlying ecosystem workflow. They are supporting context, not a prerequisite for accepting or revising this design contract.

## Failure and variation cases

- **nonzero:** Pass --fail. Expected: The process and Oyzu both exit 7.

## Contract and limitations

Acceptance criteria: ENV-04, DIST-01. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

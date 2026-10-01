# EX-046: Cross-platform profile and activation lifecycle

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- Profile installation preserves preexisting content and is idempotent.
- Activation/deactivation restores owned values without removing unrelated edits.

## Review the project

Open the checked-in project roots: `project`. Source, native manifests, and Oyzu configuration are included here so we can review the intended experience directly.

The intended Oyzu interface is:

```text
oyzu exec -- node probe.mjs
```


No build YAML is required for this scenario. Configuration, where present, demonstrates only the feature under discussion.

## Native checks available now

With the named toolchains already installed, run:

```text
# From project
node probe.mjs
```

Native commands document the underlying ecosystem workflow. They are supporting context, not a prerequisite for accepting or revising this design contract.

## Failure and variation cases

- **install-twice:** Install the shell hook twice in a temporary profile. Expected: One owned block.
- **uninstall:** Remove the Oyzu block. Expected: Original profile bytes outside that block are preserved.

## Contract and limitations

Acceptance criteria: ENV-04, ENV-05, DIST-05. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

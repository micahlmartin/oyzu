# EX-038: Idempotent publication with separate receipts

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- Identical content at an existing immutable coordinate is idempotent.
- Retry only failed operations; do not modify original manifest or claim a cross-registry transaction.

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
node --test
```

Native commands document the underlying ecosystem workflow. They are supporting context, not a prerequisite for accepting or revising this design contract.

## Failure and variation cases

- **partial-success:** First artifact succeeds, second returns 503 then succeeds. Expected: Receipt preserves both attempts; first artifact is not republished with different bytes.
- **coordinate-conflict:** Existing coordinate has a different digest. Expected: Fail conflict.

## Contract and limitations

Acceptance criteria: REL-04, BUNDLE-05. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

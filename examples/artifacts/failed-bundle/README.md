# EX-036: Failed tests and tampered artifacts remain visible

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- A failed test still produces an accurate failed bundle with available reports.
- Changing a finalized output fails digest validation; interrupted finalization is incomplete, not success.

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

- **controlled-test-failure:** Set EXAMPLE_FAIL=1 for the test action. Expected: Nonzero exit and failed tests in the bundle.
- **tamper:** Append bytes to a finalized artifact. Expected: Publishing refuses its stale digest.

## Contract and limitations

Acceptance criteria: BUNDLE-02, BUNDLE-04, BUNDLE-06. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

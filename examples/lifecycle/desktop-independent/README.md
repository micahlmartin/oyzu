# EX-045: Closing the desktop does not own the build process

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- Headless build remains valid without desktop processes.
- Attaching/closing a compatible desktop does not cancel agent/CLI-owned work; incompatible protocol reports recovery.

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

- **desktop-close:** Close the desktop during a long action. Expected: CLI-owned action continues.
- **protocol-mismatch:** Connect a deliberately incompatible desktop client. Expected: Reject without corrupting agent state.

## Contract and limitations

Acceptance criteria: DIST-01, DIST-02, DIST-03. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

# EX-039: Approved tool discovery and direct upstream acquisition

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- Protected management state selects the platform before login.
- Only allowed exact distributions resolve; package bytes use the approved direct upstream route.

## Review the project

Open the checked-in project roots: `project`. Source, native manifests, and Oyzu configuration are included here so we can review the intended experience directly.

The intended Oyzu interface is:

```text
oyzu install
oyzu exec -- node --version
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

- **denied-version:** Request a version excluded by the allowlist. Expected: Deny exact installation even with direct backend syntax.
- **missing-session:** Remove the user session while leaving protected management state. Expected: Require authentication; do not use public sources.

## Contract and limitations

Acceptance criteria: AGENT-01, CONN-05, PROTO-02. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

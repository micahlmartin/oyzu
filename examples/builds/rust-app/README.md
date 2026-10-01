# EX-021: A Cargo application with no Oyzu configuration

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- Select Cargo from native metadata, build a binary and collect test results.
- Unsupported coverage is disclosed until a pinned coverage adapter is available.

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
cargo test --locked --offline
```

Native commands document the underlying ecosystem workflow. They are supporting context, not a prerequisite for accepting or revising this design contract.

## Failure and variation cases

- **failed-test:** Change the expected greeting. Expected: Tests fail; report collection never reports success.

## Contract and limitations

Acceptance criteria: BUILDER-01, BUNDLE-03. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

# EX-021: A Cargo application with no Oyzu configuration

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- Select Cargo from native metadata, build a binary and collect test results.
- Retain native JUnit and measured application coverage alongside snapshot artifacts.

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
- **feature-gated-binary:** Replace `project/Cargo.toml` with `variants/features.Cargo.toml` and copy `variants/extra.rs` to `project/src/extra.rs`. The `extra` binary is absent from the artifact plan until the native `extra` feature is enabled. Add `default = ["extra"]` under `[features]` to include it, then remove that default to verify the next bundle excludes the prior output.
- **failed-compilation:** Introduce a native compiler error after a successful build. The new failed bundle must not reuse previous executable artifacts.
- **registry-dependency:** Build `variants/registry` as a standalone project. The locked itoa archive is acquired and checksum-verified before offline build/test/lint/format/archive execution. Only the workspace's snapshot crate and binary become outputs; JUnit and application coverage remain required. Altering the lock checksum must fail preparation.

The [Rust reference](../../../docs/reference/rust.md) distinguishes current captured-build support, native probes and remaining acceptance work. These variants add native Cargo configuration only; no Oyzu build file is needed.

## Contract and limitations

Acceptance criteria: BUILDER-01, BUNDLE-03. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

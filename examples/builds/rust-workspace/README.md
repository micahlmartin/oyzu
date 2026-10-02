# EX-022: Cargo workspace features code generation and proc macros

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- Respect Cargo workspace and feature dependencies without YAML copies.
- Capture build.rs source input; proc-macro execution stays inside the constrained toolchain context.

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
cargo test --workspace --all-features --locked --offline
```

Native commands document the underlying ecosystem workflow. They are supporting context, not a prerequisite for accepting or revising this design contract.

## Failure and variation cases

- **generated-input-change:** Edit core/message.txt. Expected: Regenerate the constant and invalidate downstream consumers.

- **doctest-failure:** Remove the `!` from the assertion in `core/src/lib.rs` documentation. Native unit tests still pass, but the documentation example fails. The build must retain a failed Cargo-invocation JUnit report and collect no artifacts. The report counts package invocations, not individual examples; doctest coverage is not claimed. See the [Rust reference](../../../docs/reference/rust.md).

- **mixed-registry:** Copy `variants/registry/core.Cargo.toml` over `project/core/Cargo.toml`, `core.rs` over `project/core/src/lib.rs`, and its Cargo-generated `Cargo.lock` over `project/Cargo.lock`. The workspace now depends on locked `itoa` as well as unpublished local crates. Expect three verified snapshot crates, one binary, test/coverage/doctest reports and unchanged inputs. No Oyzu configuration is added. Native offline host execution requires the dependency to be provisioned; captured builds acquire it through their scoped source profile.

- **incomplete-crate:** On the mixed-registry variation, add `exclude = ["message.txt"]` under the core package's `[package]` table. Workspace compilation succeeds, but native crate verification must fail because the archive omits a build-script input. No artifacts may be collected.

## Contract and limitations

Acceptance criteria: BUILDER-02, EXEC-01, PLAN-05. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

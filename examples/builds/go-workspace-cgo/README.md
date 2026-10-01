# EX-017: Go workspace with an explicit native compiler dependency

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- Discover both workspace modules and their dependency once.
- cgo requires a declared compiler and target ABI; cross-compiling is not assumed to work.

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
go test ./math/... ./cmd/...
```

Native commands document the underlying ecosystem workflow. They are supporting context, not a prerequisite for accepting or revising this design contract.

## Failure and variation cases

- **cgo-disabled:** Set CGO_ENABLED=0. Expected: Report unsupported build capability instead of substituting a different implementation.

## Contract and limitations

Acceptance criteria: BUILDER-02, EXEC-04. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

# EX-015: Legacy setup.py package with a C extension

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- Recognize legacy setuptools without evaluating arbitrary metadata on the host.
- Declare Python headers and the C compiler before isolated execution; package the platform-specific extension.

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
python -m unittest discover -s tests
```

Native commands document the underlying ecosystem workflow. They are supporting context, not a prerequisite for accepting or revising this design contract.

## Failure and variation cases

- **missing-compiler:** Execute on a worker without the declared C toolchain. Expected: Capability/preflight failure, no fallback to a random host compiler.

## Contract and limitations

Acceptance criteria: EXEC-01, EXEC-02. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).

- Native smoke tests cover the Python reference function only. Compiled extension verification requires a C toolchain and tests/native_check.py after building.

Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

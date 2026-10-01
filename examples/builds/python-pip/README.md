# EX-013: Requirements-based application

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- Discover pip requirements without synthesizing package metadata.
- Prepare pinned hashed requirements; a requirements-only script must not silently become a wheel.
- The entrypoint may need a minimal app hint if discovery is ambiguous.

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
python -m pip install -r requirements.txt
python -m unittest discover -s tests
```

Native commands document the underlying ecosystem workflow. They are supporting context, not a prerequisite for accepting or revising this design contract.

## Failure and variation cases

- **unhashed-requirement:** Remove the wheel hash from requirements.txt. Expected: Disclose incomplete reproducibility and enforce applicable locked-input policy.

## Contract and limitations

Acceptance criteria: EXEC-02, BUILDER-04. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).

- The baseline pins packaging with its published wheel digest. unlocked-requirements.txt is the deliberately incomplete-lock negative input. The digest comes from the [PyPI release metadata](https://pypi.org/pypi/packaging/24.2/json).

Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

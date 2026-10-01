# EX-011: A Python library with uv and automatic reports

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- Discover uv from its generated lock and package metadata; no Oyzu configuration.
- Produce wheel/sdist and collect pytest JUnit and coverage via the builder.

## Review the project

Open the checked-in project roots: `project`. Source, native manifests, and Oyzu configuration are included here so we can review the intended experience directly.

The intended Oyzu interface is:

```text
oyzu build
oyzu run test
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

- **unlocked-build-requirement:** Change the pinned build backend without preparing inputs. Expected: Frozen preparation fails; no download from inside an action.

## Contract and limitations

Acceptance criteria: BUILDER-01, EXEC-02, BUNDLE-01. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

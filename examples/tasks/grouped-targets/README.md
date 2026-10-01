# EX-008: Qualified tasks in a mixed target build

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- api:test and web:test use their own working directories and tools.
- An unqualified ambiguous test from the root reports both choices.

## Review the project

Open the checked-in project roots: `project`. Source, native manifests, and Oyzu configuration are included here so we can review the intended experience directly.

The intended Oyzu interface is:

```text
oyzu run list
oyzu run api:test
oyzu run web:test
```


The build YAML uses draft OEP syntax and contains only target intent/relationships. It is not a stable schema.

## Native checks available now

With the named toolchains already installed, run:

```text
# From project/web
node --test

# From project/api
python -m unittest discover -s tests
```

Native commands document the underlying ecosystem workflow. They are supporting context, not a prerequisite for accepting or revising this design contract.

## Failure and variation cases

- **ambiguous-root:** Run oyzu run test from the multi-target root. Expected: Ambiguity error, not an arbitrary target.

## Contract and limitations

Acceptance criteria: TASK-06, PLAN-02. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

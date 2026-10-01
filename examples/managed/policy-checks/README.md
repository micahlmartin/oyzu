# EX-043: Change mandatory scanning without editing a repository

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- The same source under two policy revisions has different required checks.
- A client unable to enforce the new required scanner rejects the plan.

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

- **scanner-required:** Select revision two. Expected: Plan includes the required check with pinned rules/database input.
- **unsupported-client:** Remove scanner capability from client advertisement. Expected: Explicit update/capability error.

## Contract and limitations

Acceptance criteria: PROTO-03, PLAN-06. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

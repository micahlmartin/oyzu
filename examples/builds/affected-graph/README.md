# EX-033: Shared input and transitive affected targets

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- A shared input change affects API and web; an API-only change leaves web unaffected.
- Unknown undeclared dependencies force a conservative full rebuild.

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

- **shared-change:** Edit shared/message.mjs. Expected: API and web tests/actions are affected.
- **api-change:** Edit api/index.mjs. Expected: API changes; unrelated web action remains reusable.

## Contract and limitations

Acceptance criteria: PLAN-05, CACHE-02. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

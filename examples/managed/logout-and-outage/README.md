# EX-041: Management persists through logout and outages

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- Logout removes session access, not mandatory management.
- Cached permission works only within explicit scope and validity; expiry fails closed.

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

- **agent-offline:** Stop the agent. Expected: Native package operations fail, no public source fallback.
- **expired-snapshot:** Advance beyond snapshot validity. Expected: No authorization based on stale cached metadata.

## Contract and limitations

Acceptance criteria: CFG-02, AGENT-04, PROTO-05. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

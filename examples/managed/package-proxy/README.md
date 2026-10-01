# EX-040: Local package routes and expiring upstream leases

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- Native npm/pip use local endpoints; the agent obtains a scoped upstream lease.
- Neither native environment nor package config contains the upstream token; redirects cannot exfiltrate it.

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

- **expired-token:** Expire the upstream lease between requests. Expected: Refresh with approved scope or fail; no static privileged fallback.
- **redirect:** Return an upstream redirect to an unapproved host. Expected: Reject without forwarding Authorization.
- **wrong-scope:** Request a different repository. Expected: Deny.

## Contract and limitations

Acceptance criteria: AGENT-02, AGENT-03, CONN-03. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

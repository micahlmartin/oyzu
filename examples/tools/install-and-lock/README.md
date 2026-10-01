# EX-001: Install a tool and preserve its identity

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- Resolve node 22.14.0 to a platform-specific distribution and generate oyzu.lock.
- Repeated installation preserves the exact version and digest; a revoked cached distribution is denied.

## Review the project

Open the checked-in project roots: `project`. Source, native manifests, and Oyzu configuration are included here so we can review the intended experience directly.

The intended Oyzu interface is:

```text
oyzu install
oyzu exec -- node probe.mjs
```


No build YAML is required for this scenario. Configuration, where present, demonstrates only the feature under discussion.

## Native checks available now

With the named toolchains already installed, run:

```text
# From project
node probe.mjs
```

Native commands document the underlying ecosystem workflow. They are supporting context, not a prerequisite for accepting or revising this design contract.

## Failure and variation cases

- **mutable-upstream:** Serve different bytes under the locked version. Expected: Integrity error; the installed tool and lock remain unchanged.

## Contract and limitations

Acceptance criteria: TOOL-01, TOOL-02. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

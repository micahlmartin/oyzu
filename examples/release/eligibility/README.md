# EX-037: Version text and branch labels do not authorize production

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- Resolve versions using ecosystem rules and verified source facts.
- A protected branch or release-looking version cannot make a workstation artifact production-eligible.

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

- **spoofed-ci:** Set CI=true locally. Expected: Still local/unverified.
- **protected-local:** Build locally from a protected ref. Expected: Production denied.

## Contract and limitations

Acceptance criteria: REL-01, REL-02, REL-03. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

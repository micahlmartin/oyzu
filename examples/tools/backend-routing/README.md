# EX-047: All backend acquisition paths must be mediated

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- No separate mise executable is launched.
- Direct HTTP, Git, and package-manager backend paths all use approved acquisition or report unsupported managed capability.

## Review the project

Open the checked-in project roots: `project`. Source, native manifests, and Oyzu configuration are included here so we can review the intended experience directly.

The intended Oyzu interface is:

```text
oyzu install
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

- **subprocess-bypass:** Backend invokes a child downloader outside the broker. Expected: Blocked by managed acquisition boundary.
- **alias-bypass:** Use explicit backend syntax for a denied tool. Expected: Same canonical identity denial.

## Contract and limitations

Acceptance criteria: MISE-01, MISE-04, TOOL-04. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

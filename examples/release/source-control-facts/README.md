# EX-048: Provider facts retain unknown states

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- GitHub/GitLab protection responses do not grant release authority on their own.
- Permission denied, fork-origin events, and unverified workload identity cannot be interpreted as trusted execution.

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

- **permission-denied:** Source-control API returns 403. Expected: Protection unknown; required positive evidence absent.
- **fork-pr:** Use an untrusted fork PR event with CI variables. Expected: No production authority from the label.

## Contract and limitations

Acceptance criteria: CONN-06, REL-02, REL-05. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

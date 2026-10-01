# EX-034: OCI cache configuration without pipeline cache scripts

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- A standalone user configures only an OCI destination.
- Cold build stores complete blobs/results; warm build restores verified content; unavailable cache falls back to computation.

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

- **interrupted-write:** Disconnect before result manifest commit. Expected: No reusable partial entry.
- **concurrent-writers:** Publish incompatible outputs for the same action key. Expected: Record conflict; never silently overwrite trusted results.

## Contract and limitations

Acceptance criteria: CACHE-01, CACHE-03, CACHE-06. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

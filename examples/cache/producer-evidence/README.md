# EX-035: A local cache entry cannot confer release authority

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- A consuming CI run preserves original output producer facts.
- Uploading or signing local content later does not change its local origin.

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

- **local-to-ci:** Reuse a local producer descriptor in CI. Expected: Deny production eligibility even when output bytes match.
- **unverified-signature:** Attach a client-created signature. Expected: A signature without verified authority is not sufficient.

## Contract and limitations

Acceptance criteria: CACHE-04, REL-06. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

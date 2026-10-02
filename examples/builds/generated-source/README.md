# EX-031: A shared deterministic generator with two consumers

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- Capture schema and generator as declared inputs.
- Generate once; both consumers depend on the generated module's content.

## Review the project

Open the checked-in project roots: `project`. Source, native manifests, and Oyzu configuration are included here so we can review the intended experience directly.

The intended Oyzu interface is:

```text
oyzu build
```


No build YAML is required for this scenario. Configuration, where present, demonstrates only the feature under discussion.

## Native checks available now

The `generated-source` check starts the `node` captured-build suite. It uses this project without adding configuration, requires the generated module in the snapshot package plus both consumers' JUnit and coverage, and checks that changed schema data reaches both tests. See [acceptance setup and current scope](../../../docs/reference/build-verification.md). The compiled Windows CLI passes the native build/test/lint/format-check flow; captured Linux execution is pending. This first proof does not establish the selective cache invalidation case below or separate generator action identities.

With the named toolchains already installed, run:

```text
# From project
node generate.mjs

# From project
node --test
```

Native commands document the underlying ecosystem workflow. They are supporting context, not a prerequisite for accepting or revising this design contract.

## Failure and variation cases

- **schema-change:** Change schema.json's greeting. Expected: Only the generator and affected consumer actions invalidate.

## Contract and limitations

Acceptance criteria: PLAN-06, CACHE-02. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

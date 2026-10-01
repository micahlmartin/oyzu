# EX-009: Replace an implicit task and attach hooks

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- pre_test → test → post_test in both entrypoints.
- Pre failure prevents main; main failure skips post; post failure fails the invocation.
- Overrides retain the builder report contract.

## Review the project

Open the checked-in project roots: `project`. Source, native manifests, and Oyzu configuration are included here so we can review the intended experience directly.

The intended Oyzu interface is:

```text
oyzu run api:test
oyzu build
```


The build YAML uses draft OEP syntax and contains only target intent/relationships. It is not a stable schema.

## Native checks available now

With the named toolchains already installed, run:

```text
# From project/api
node --test

# From project/api
node hook.mjs pre
```

Native commands document the underlying ecosystem workflow. They are supporting context, not a prerequisite for accepting or revising this design contract.

## Failure and variation cases

- **pre-fails:** Use FAILURE_POINT=pre in a disposable development invocation. Expected: Only pre event recorded.
- **main-fails:** Use FAILURE_POINT=main. Expected: Pre and main recorded, no post.
- **post-fails:** Use FAILURE_POINT=post. Expected: All events recorded and invocation fails.

## Contract and limitations

Acceptance criteria: TASK-02, TASK-03, TASK-04. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

# EX-010: Hooks around cached computation

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- Noncacheable hooks run on a cache hit.
- Changing hook-produced input invalidates the dependent action.

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
node hook.mjs pre
node --test

```

Native commands document the underlying ecosystem workflow. They are supporting context, not a prerequisite for accepting or revising this design contract.

## Failure and variation cases

- **pre-fails:** Use FAILURE_POINT=pre in a disposable development invocation. Expected: Only pre event recorded.
- **main-fails:** Use FAILURE_POINT=main. Expected: Pre and main recorded, no post.
- **post-fails:** Use FAILURE_POINT=post. Expected: All events recorded and invocation fails.

## Contract and limitations

The `inputs` and `outputs` arrays are candidate syntax for the case where a custom hook produces a main-task input. The pre-hook remains noncacheable by default; the main task may reuse results only after the generated input digest is known. Review whether these declarations are the smallest necessary exception.

Acceptance criteria: TASK-05, CACHE-02. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

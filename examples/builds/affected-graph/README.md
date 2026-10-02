# EX-033: Shared input and transitive affected targets

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- A shared input change affects API and web; an API-only change leaves web unaffected.
- Unknown undeclared dependencies force a conservative full rebuild.

## Review the project

Open the checked-in project roots: `project`. Source, native manifests, and Oyzu configuration are included here so we can review the intended experience directly.

The intended Oyzu interface is:

```text
oyzu build
```


The original `project` needs no build YAML and specifies intra-project action reuse, which remains unfinished. `variants/targets` separately demonstrates implemented target-level Git selection with four declared Node packages: shared, api, web and unused. Both api and web depend on shared.

From that variation, create a local Git commit, then run `oyzu build --affected HEAD`. No changes produce an empty inspected bundle. Editing `api/src/greeting.mjs` selects api plus its shared prerequisite; editing `shared/src/greeting.mjs` selects shared and both consumers. Each selected target receives its required checks and snapshot package. See [affected build behavior and limits](../../../docs/reference/affected-builds.md).

The compiled Windows CLI passed the no-change flow. Actual selective artifact checks are registered in the Linux Node suite and await CI results. This does not establish cache reuse or mark the original scenario complete.

## Native checks available now

With the named toolchains already installed, run:

```text
# From project
node --test
```

Native commands document the underlying ecosystem workflow. They are supporting context, not a prerequisite for accepting or revising this design contract.

## Failure and variation cases

- **shared-change:** Edit shared/message.mjs. Expected: API and web tests/actions are affected.
- **api-change:** Edit api/index.mjs. Expected: API changes; unrelated web action remains reusable.

## Contract and limitations

Acceptance criteria: PLAN-05, CACHE-02. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

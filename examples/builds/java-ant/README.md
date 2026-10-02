# EX-025: Conventional and custom Ant task names

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- Infer conventional compile/test/jar where semantics are known.
- The custom target uses a single task override; no attempt to reverse-engineer arbitrary Ant behavior.
- A Java main assertion is not a JUnit report; missing report support stays visible.

## Review the project

Open the checked-in project roots: `conventional`, `custom`. Source, native manifests, and Oyzu configuration are included here so we can review the intended experience directly.

The intended Oyzu interface is:

```text
oyzu run list
oyzu run test
oyzu build
```


No build YAML is required for this scenario. Configuration, where present, demonstrates only the feature under discussion.

## Native checks available now

With the named toolchains already installed, run:

```text
# From conventional
ant test jar

# From custom
ant verify-contract jar
```

Native commands document the underlying ecosystem workflow. They are supporting context, not a prerequisite for accepting or revising this design contract.

## Failure and variation cases

- **unknown-task:** Rename verify-contract without updating the override. Expected: A precise missing-target failure.

## Contract and limitations

Acceptance criteria: BUILDER-04, TASK-01. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

## Java quality gates

The [Java quality reference](../../../docs/reference/java-quality.md) describes provisioned native lint and formatting defaults. `oyzu run list` includes `lint`, `format-check` and explicit `format`; builds run the read-only checks before exporting artifacts. Add an unused import or a formatting-only change to exercise failure: test evidence remains available, artifacts are blocked and the checkout stays unchanged. Native task replacements retain their existing authority.

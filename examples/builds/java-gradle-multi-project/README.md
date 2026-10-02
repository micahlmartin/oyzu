# EX-024: Gradle subprojects and an included build

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- Respect subprojects plus composite-build dependency substitution.
- Prepare Gradle and JDK before execution; wrapper-generated downloads must obey acquisition routing.

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
gradle --no-daemon test jar
```

Native commands document the underlying ecosystem workflow. They are supporting context, not a prerequisite for accepting or revising this design contract.

## Failure and variation cases

- **wrapper-request:** Generate a wrapper for an approved Gradle distribution. Expected: Validate distribution checksum and route; wrapper execution cannot bypass managed acquisition.

## Contract and limitations

Acceptance criteria: TOOL-01, BUILDER-02. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).

- Installed-Gradle fixture; wrapper generation/integrity variant is specified but not checked in or verified.

Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

## Java quality gates

The [Java quality reference](../../../docs/reference/java-quality.md) describes provisioned native lint and formatting defaults. `oyzu run list` includes `lint`, `format-check` and explicit `format`; builds run the read-only checks before exporting artifacts. Add an unused import or a formatting-only change to exercise failure: test evidence remains available, artifacts are blocked and the checkout stays unchanged. Native task replacements retain their existing authority.

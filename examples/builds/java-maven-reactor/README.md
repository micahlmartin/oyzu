# EX-023: Maven multi-module reactor

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- Use the native parent/modules/dependency graph once.
- Associate library and app jars plus Surefire reports with their modules.

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
mvn --batch-mode verify
```

Native commands document the underlying ecosystem workflow. They are supporting context, not a prerequisite for accepting or revising this design contract.

## Failure and variation cases

- **shared-code-change:** Change the library greeting. Expected: Re-test the consuming app through the reactor graph.

- **native-reports:** Copy `variants/reporting/app.pom.xml` to `project/app/pom.xml` and `AppIT.java` to `project/app/src/test/java/example/AppIT.java`. Maven now runs Surefire and Failsafe with custom report directories. Expect all three test reports, module coverage and snapshot JAR/POM artifacts without extra Oyzu configuration. See the [Maven reference](../../../docs/reference/maven.md).
- **integration-failure:** Change the expected greeting in `AppIT.java`; retain the native failed integration-test XML and block artifacts.
- **integration-only:** Remove `AppTest.java` from the reporting variation. The app has only integration tests; absence of a Surefire report must not prevent collecting its successful Failsafe evidence.

## Contract and limitations

Acceptance criteria: BUILDER-02, BUNDLE-01. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

## Java quality gates

The [Java quality reference](../../../docs/reference/java-quality.md) describes provisioned native lint and formatting defaults. `oyzu run list` includes `lint`, `format-check` and explicit `format`; builds run the read-only checks before exporting artifacts. Add an unused import or a formatting-only change to exercise failure: test evidence remains available, artifacts are blocked and the checkout stays unchanged. Native task replacements retain their existing authority.

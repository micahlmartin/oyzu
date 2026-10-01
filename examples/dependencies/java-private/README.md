# EX-053: Private Maven Gradle and Ant Ivy dependency preparation

Status: **design contract for review; implementation pending**. Read the [shared acquisition contract](../../DEPENDENCIES.md) and [validation scope](../../VERIFICATION.md).

## Developer experience

Open `maven`, `gradle`, `ant-ivy`. Each is a separate project unless stated otherwise below. Run `oyzu build` from the chosen root. Native manifests identify the manager and dependencies; no project-level secret or registry-authentication section is required.

Standalone users configure approved registry mappings once outside the repository. Managed users receive them from policy. The same source runs in either profile. Review the native package/app output first, then its container packaging using the same captured dependencies; image selection syntax remains a separate design question.

## Inputs Oyzu must prepare

- Application dependencies plus plugins, annotation processors, parent POMs and buildscript repositories.
- JDKs, pinned build tools, wrapper distributions where present, and Ivy itself for the Ant fixture.

The fixture JAR provides `invalid.example.oyzu.Greeting.message()`. Repository URLs are routing inputs, not credentials. Standalone mappings or managed policy provide the Maven mirror and Ivy resolver; Gradle repository declarations must obey the same routing boundary, including plugin repositories. No global developer settings are imported. Ant alone is not a dependency manager: this fixture specifically uses Ivy and requires explicit adapter recognition; arbitrary Ant download targets remain unsupported until described.

## Failure and variation cases

- **private-plugin:** Add a private Maven plugin or Gradle buildscript dependency. Expected: Capture it as a build dependency, not just the application runtime closure.
- **ant-get:** Add an Ant get task downloading from an undeclared URL. Expected: Deny in execution with the owning target identified; do not treat arbitrary Ant tasks as known dependency resolution.

All scenarios additionally cover denied managed routes, incomplete captures, credential observation, and expired authorization in [scenario.json](scenario.json). [Expected acquisition](expected-acquisition.json) records the common boundary and evidence outside project configuration.

## Fixture limits

Private package names and `example.invalid` URLs are synthetic fixture inputs, not live services. The future harness must serve minimal packages matching these manifests and generate native locks and real digests; no fake checksums, successful build records, credentials or registry infrastructure are included. Missing locks are preparation work or a policy error, never permission for unrecorded online execution. `nativeChecks` is empty because these private-dependency projects cannot currently resolve against a fixture registry.

Acceptance: EXEC-02, EXEC-07, BUILDER-08, CONN-03. These files demonstrate the intended design, not verified package-manager support. Host and target support require separate execution evidence.

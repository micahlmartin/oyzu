# EX-051: Private Python dependencies across uv pip Poetry and legacy projects

Status: **design contract for review; implementation pending**. Read the [shared acquisition contract](../../DEPENDENCIES.md) and [validation scope](../../VERIFICATION.md).

## Developer experience

Open `uv`, `pip`, `poetry`, `legacy`. Each is a separate project unless stated otherwise below. Run `oyzu build` from the chosen root. Native manifests identify the manager and dependencies; no project-level secret or registry-authentication section is required.

Standalone users configure approved registry mappings once outside the repository. Managed users receive them from policy. The same source runs in either profile. Review the native package/app output first, then its container packaging using the same captured dependencies; image selection syntax remains a separate design question.

## Inputs Oyzu must prepare

- Runtime packages, metadata and transitive dependencies from the approved Python repository.
- PEP 517 build requirements, source distributions, native headers/compilers and target-compatible wheels where required.

The fixture package exports `greeting()`. A wheelhouse or manager-specific prepared store is an internal choice. A host virtualenv must not be copied into a different target runtime. Legacy metadata execution is isolated and has no registry credentials. The manager markers in pyproject files express existing native intent; the future fixture locks disambiguate selection.

## Failure and variation cases

- **sdist-build-dependency:** Serve a source distribution whose build backend needs another private package. Expected: Capture that requirement through controlled preparation, then build the wheel without credentials or action networking.
- **wrong-wheel:** Provide only a workstation wheel for a different container ABI. Expected: Acquire/build a compatible wheel with captured toolchain inputs or fail.

All scenarios additionally cover denied managed routes, incomplete captures, credential observation, and expired authorization in [scenario.json](scenario.json). [Expected acquisition](expected-acquisition.json) records the common boundary and evidence outside project configuration.

## Fixture limits

Private package names and `example.invalid` URLs are synthetic fixture inputs, not live services. The future harness must serve minimal packages matching these manifests and generate native locks and real digests; no fake checksums, successful build records, credentials or registry infrastructure are included. Missing locks are preparation work or a policy error, never permission for unrecorded online execution. `nativeChecks` is empty because these private-dependency projects cannot currently resolve against a fixture registry.

Acceptance: EXEC-02, EXEC-07, BUILDER-08, CONN-03. These files demonstrate the intended design, not verified package-manager support. Host and target support require separate execution evidence.

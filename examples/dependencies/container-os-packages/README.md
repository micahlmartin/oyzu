# EX-057: Prepared apt and apk inputs for container images

Status: **design contract for review; implementation pending**. Read the [shared acquisition contract](../../DEPENDENCIES.md) and [validation scope](../../VERIFICATION.md).

## Developer experience

Open `apt`, `apk`. Each is a separate project unless stated otherwise below. Run `oyzu build` from the chosen root. Native manifests identify the manager and dependencies; no project-level secret or registry-authentication section is required.

Standalone users configure approved repository mappings once outside the repository. Managed users receive them from policy. The same source runs in either profile. Review the image and OS package closure. The future harness serves approved repository snapshots through a synthetic authenticated route; it does not use the distro's public repository as a fallback.

## Inputs Oyzu must prepare

- Base images resolved to immutable digests before execution.
- Approved distro repository metadata, signing roots, exact package closure and target-compatible archives, including prerequisites of installation scripts.

These Dockerfiles deliberately show the familiar network-dependent starting point, not an already-hermetic implementation. A supported, explicitly enabled integration must prepare repository inputs and arrange offline installation with reviewable plan details; otherwise Oyzu rejects the network-dependent RUN with a useful explanation. It must not silently rewrite arbitrary shell commands. apt and apk are separate adapters, not interchangeable archive formats. Exact author-facing integration syntax remains open. Base tags are fixture inputs resolved and frozen during preparation, not a claim that tags are immutable.

## Failure and variation cases

- **expired-repository-metadata:** Provide expired or invalidly signed repository metadata. Expected: Enforce freshness and signature policy; do not disable verification to make installation succeed.
- **maintainer-script-download:** A package maintainer script attempts an undeclared network fetch. Expected: Deny the fetch; require captured supported inputs or explain the unsupported package behavior.
- **unsupported-dockerfile-transform:** Wrap package installation in arbitrary shell logic the adapter cannot safely interpret. Expected: Explain the unsupported preparation path without running online or silently modifying the source Dockerfile.

All scenarios additionally cover denied managed routes, incomplete captures, credential observation, and expired authorization in [scenario.json](scenario.json). [Expected acquisition](expected-acquisition.json) records the common boundary and evidence outside project configuration.

## Fixture limits

Private package names and `example.invalid` URLs are synthetic fixture inputs, not live services. The future harness must serve minimal packages matching these manifests and generate native locks and real digests; no fake checksums, successful build records, credentials or registry infrastructure are included. Missing locks are preparation work or a policy error, never permission for unrecorded online execution. `nativeChecks` is empty because these private-dependency projects cannot currently resolve against a fixture registry.

Acceptance: EXEC-02, EXEC-07, BUILDER-08, CONN-03. These files demonstrate the intended design, not verified package-manager support. Host and target support require separate execution evidence.

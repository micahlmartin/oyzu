# EX-056: Private Helm dependencies and OCI registry authentication

Status: **design contract for review; implementation pending**. Read the [shared acquisition contract](../../DEPENDENCIES.md) and [validation scope](../../VERIFICATION.md).

## Developer experience

Open `project`. Each is a separate project unless stated otherwise below. Run `oyzu build` from the chosen root. Native manifests identify the manager and dependencies; no project-level secret or registry-authentication section is required.

Standalone users configure approved registry mappings once outside the repository. Managed users receive them from policy. The same source runs in either profile. Review offline chart dependency assembly, rendering and packaging. Helm produces a chart artifact, not a container image; it shares the credential boundary with image builds.

## Inputs Oyzu must prepare

- Chart metadata, locked dependency versions and verified chart archives.
- OCI manifest/blob digests or HTTP repository metadata as applicable; no deployment credentials.

The future registry serves a minimal Helm library chart named fixture-library. The variant replaces only Chart.yaml to exercise HTTP repository authentication. Packaging and rendering receive prepared charts without registry login files. Read access for dependency acquisition does not imply chart or image publication rights.

## Failure and variation cases

- **publish-with-read-lease:** Attempt publication using the dependency-acquisition authorization. Expected: Deny; obtain separately scoped publication authorization through the publication flow.
- **mutable-chart:** Serve different bytes for a previously captured chart version. Expected: Reject against the frozen digest; version text alone cannot validate the dependency.

All scenarios additionally cover denied managed routes, incomplete captures, credential observation, and expired authorization in [scenario.json](scenario.json). [Expected acquisition](expected-acquisition.json) records the common boundary and evidence outside project configuration.

## Fixture limits

Private package names and `example.invalid` URLs are synthetic fixture inputs, not live services. The future harness must serve minimal packages matching these manifests and generate native locks and real digests; no fake checksums, successful build records, credentials or registry infrastructure are included. Missing locks are preparation work or a policy error, never permission for unrecorded online execution. `nativeChecks` is empty because these private-dependency projects cannot currently resolve against a fixture registry.

Acceptance: EXEC-02, EXEC-07, BUILDER-08, CONN-03. These files demonstrate the intended design, not verified package-manager support. Host and target support require separate execution evidence.

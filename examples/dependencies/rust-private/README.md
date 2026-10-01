# EX-055: Private Cargo registries and credential-free build scripts

Status: **design contract for review; implementation pending**. Read the [shared acquisition contract](../../DEPENDENCIES.md) and [validation scope](../../VERIFICATION.md).

## Developer experience

Open `project`. Each is a separate project unless stated otherwise below. Run `oyzu build` from the chosen root. Native manifests identify the manager and dependencies; no project-level secret or registry-authentication section is required.

Standalone users configure approved registry mappings once outside the repository. Managed users receive them from policy. The same source runs in either profile. Review the native package/app output first, then its container packaging using the same captured dependencies; image selection syntax remains a separate design question.

## Inputs Oyzu must prepare

- Registry index metadata, crate archives and checksum information for the locked graph.
- Git dependencies, build dependencies, proc macros and target/native toolchains.

The fixture crate exports `greeting()`. A native registry alias is needed because the package uses an alternate registry; it contains no token. Credential providers or broker routing are adapter concerns. The negative build-script probe is a narrow observation, not proof that every credential channel is closed.

## Failure and variation cases

- **build-script-credential:** Install variants/build.rs in the app and inject an upstream token into Cargo execution. Expected: The future harness must detect this boundary violation; the supported path never injects that token.
- **crate-download-redirect:** Registry metadata points a crate download to an unapproved domain. Expected: Reject or explicitly approve a credential-free redirected route; never forward the original Authorization header.

All scenarios additionally cover denied managed routes, incomplete captures, credential observation, and expired authorization in [scenario.json](scenario.json). [Expected acquisition](expected-acquisition.json) records the common boundary and evidence outside project configuration.

## Fixture limits

Private package names and `example.invalid` URLs are synthetic fixture inputs, not live services. The future harness must serve minimal packages matching these manifests and generate native locks and real digests; no fake checksums, successful build records, credentials or registry infrastructure are included. Missing locks are preparation work or a policy error, never permission for unrecorded online execution. `nativeChecks` is empty because these private-dependency projects cannot currently resolve against a fixture registry.

Acceptance: EXEC-02, EXEC-07, BUILDER-08, CONN-03. These files demonstrate the intended design, not verified package-manager support. Host and target support require separate execution evidence.

# EX-054: Private Go modules without direct VCS fallback

Status: **design contract for review; implementation pending**. Read the [shared acquisition contract](../../DEPENDENCIES.md) and [validation scope](../../VERIFICATION.md).

## Developer experience

Open `project`. Each is a separate project unless stated otherwise below. Run `oyzu build` from the chosen root. Native manifests identify the manager and dependencies; no project-level secret or registry-authentication section is required.

Standalone users configure approved registry mappings once outside the repository. Managed users receive them from policy. The same source runs in either profile. Review the native package/app output first, then its container packaging using the same captured dependencies; image selection syntax remains a separate design question.

## Inputs Oyzu must prepare

- Module metadata, archives and integrity information through the approved module proxy.
- Approved private VCS sources where explicitly supported and captured at immutable commits; target toolchain and cgo inputs when needed.

The fixture module exports `greeting.Message()`. Capture the native go.sum and graph before isolated execution. Private-module routing and checksum settings must avoid both public disclosure and accidental direct VCS fallback; privacy configuration must not remove content verification. The agent, not the compiled application or Go build scripts, owns upstream authentication.

## Failure and variation cases

- **proxy-404:** The approved proxy returns 404 for a private module. Expected: Fail without falling back to public proxies or a direct Git connection.
- **private-checksum-leak:** Resolve a private module whose path is unknown to public infrastructure. Expected: Do not send its path to the public checksum service; retain verified captured content digests.

All scenarios additionally cover denied managed routes, incomplete captures, credential observation, and expired authorization in [scenario.json](scenario.json). [Expected acquisition](expected-acquisition.json) records the common boundary and evidence outside project configuration.

## Fixture limits

Private package names and `example.invalid` URLs are synthetic fixture inputs, not live services. The future harness must serve minimal packages matching these manifests and generate native locks and real digests; no fake checksums, successful build records, credentials or registry infrastructure are included. Missing locks are preparation work or a policy error, never permission for unrecorded online execution. `nativeChecks` is empty because these private-dependency projects cannot currently resolve against a fixture registry.

Acceptance: EXEC-02, EXEC-07, BUILDER-08, CONN-03. These files demonstrate the intended design, not verified package-manager support. Host and target support require separate execution evidence.

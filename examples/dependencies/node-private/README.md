# EX-052: Private npm pnpm and Yarn dependencies with lifecycle scripts

Status: **design contract for review; implementation pending**. Read the [shared acquisition contract](../../DEPENDENCIES.md) and [validation scope](../../VERIFICATION.md).

## Developer experience

Open `npm`, `pnpm`, `yarn`. Each is a separate project unless stated otherwise below. Run `oyzu build` from the chosen root. Native manifests identify the manager and dependencies; no project-level secret or registry-authentication section is required.

Standalone users configure approved registry mappings once outside the repository. Managed users receive them from policy. The same source runs in either profile. Review the native package/app output first, then its container packaging using the same captured dependencies; image selection syntax remains a separate design question.

## Inputs Oyzu must prepare

- Package metadata, archives, integrity records and the selected manager distribution.
- Native-addon toolchains and any supported lifecycle assets, including browser binaries, captured separately from script execution.

The fixture package exports `greeting()` returning `hello`. Preserve manager-specific layouts, including Yarn PnP where selected; do not assume all managers produce node_modules. The negative lifecycle script lives outside the positive projects and is attached to a synthetic dependency only by the future harness.

## Failure and variation cases

- **lifecycle-download:** Use variants/download-browser.mjs as a dependency install script. Expected: Reject its network access in execution. A supported adapter may prepare the declared browser asset first; an unknown download needs a diagnostic, not an internet retry.
- **git-dependency:** Replace the registry dependency with an approved private Git dependency. Expected: Use mediated source acquisition and a frozen commit; npm registry credentials must not be forwarded to Git.

All scenarios additionally cover denied managed routes, incomplete captures, credential observation, and expired authorization in [scenario.json](scenario.json). [Expected acquisition](expected-acquisition.json) records the common boundary and evidence outside project configuration.

## Fixture limits

Private package names and `example.invalid` URLs are synthetic fixture inputs, not live services. The future harness must serve minimal packages matching these manifests and generate native locks and real digests; no fake checksums, successful build records, credentials or registry infrastructure are included. Missing locks are preparation work or a policy error, never permission for unrecorded online execution. `nativeChecks` is empty because these private-dependency projects cannot currently resolve against a fixture registry.

Acceptance: EXEC-02, EXEC-07, BUILDER-08, CONN-03. These files demonstrate the intended design, not verified package-manager support. Host and target support require separate execution evidence.

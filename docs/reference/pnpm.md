# pnpm builds

Oyzu recognizes a conventional Node project using pnpm and its `pnpm-lock.yaml`. Single projects and conventional native workspaces need no Oyzu build configuration. The experimental adapter captures public-registry dependency archives, verifies their SHA-512 lockfile integrity, and uses native pnpm for frozen-lock validation and installation. Native pnpm retains responsibility for dependency layout and peer resolution.

## Prerequisites and use

The current provisioned profile is pnpm 10.11.0 on Node 22. Build the common quality image, then the pnpm image:

```text
docker build -t oyzu-toolchain/node:quality tooling/images/node-quality
docker build -t oyzu-toolchain/node:pnpm10.11.0-node22 tooling/images/node-pnpm
oyzu --root path/to/project run list
oyzu --root path/to/project build
oyzu --root path/to/project inspect dist
```

The build requires Docker and these explicitly provisioned images; Oyzu does not install them. `--image pnpm=<image>` selects an already provisioned compatible image. The image must include the owned runtime prerequisites, including `yaml` 2.8.1 at `/opt/oyzu-pnpm/node_modules/yaml`, and the Node quality tools. Its immutable image identity binds those tools. A project's exact `packageManager`, when present, must agree with native pnpm's version. Native strict engine validation rejects incompatible Node requirements.

Development tasks use installed host tools. `oyzu run list` exposes native package scripts and inferred test/quality tasks; `oyzu run <task>` uses the development environment. Captured builds run native build/test work and read-only lint/format checks. Node's native test runner receives JUnit and LCOV reporting through the existing Node builder; other supported framework reporting remains framework-specific. See [Node quality](node-quality.md) for quality selection and provisioned defaults. Formatting during a build never rewrites the checkout.

## Acquisition and execution

A finite `matrix.node` in `build.yaml` selects exact provisioned runtimes while keeping native pnpm scripts, frozen locks and patches unchanged. Each variant records its actual Node version and receives separate tasks, reports and snapshot artifacts. See [runtime matrices](runtime-matrices.md) for image naming, compatibility checks, limits and measured verification. Workspace support is described below; private registry configuration remains unfinished.

The admitted lock format is v9 with a root importer and, for workspaces, native member importers. Every registry package in its `packages` section is captured, including transitive and optional entries, even when native pnpm does not install an entry on the selected platform. Resolution must contain a SHA-512 integrity value. Registry archive URLs derive from the locked package name/version and use the engine's scoped `npm-public` route to `https://registry.npmjs.org/`.

Acquisition disables project lifecycle scripts. Unsupported pnpm configuration and hook files are rejected before any native invocation, including the version query. Preparation verifies archive digests before handing bytes to pnpm. The snapshot contains content-addressed tarballs and a deterministic inventory; it does not retain pnpm's timestamped mutable store.

For installation, the adapter creates a fresh private store and serves the exact captured archive allowlist on an ephemeral loopback port. The isolated worker has no external network. This server never forwards requests or holds registry credentials; unknown routes fail. Native pnpm needs HTTP access to this local server, so its `--offline` switch is not used. The worker isolation and archive allowlist enforce the external-network boundary. Execution mounts no acquisition broker, rechecks archive identity, and enables native lifecycle behavior. pnpm 10's dependency-script approval rules still apply; Oyzu does not silently approve dependency scripts.

Native lock/manifest checks run with `--frozen-lockfile`. The adapter checks that the lock bytes remain unchanged and requires the same pnpm/Node versions during preparation and execution. Credentials are absent from the inventory and loopback endpoint. Managed/private registry routes remain unimplemented for this profile.

## Native dependency patches

Checked-in `patchedDependencies` settings are supported in a single project's `pnpm-workspace.yaml` or the native `pnpm` object in `package.json`. For example:

```yaml
patchedDependencies:
  is-number@6.0.0: patches/is-number@6.0.0.patch
```

Use native [pnpm patch](https://pnpm.io/10.x/cli/patch) and [patch-commit](https://pnpm.io/10.x/cli/patch-commit) to generate the patch, settings and updated lock; commit those inputs together. Oyzu needs no additional setting. A `pnpm-workspace.yaml` containing these settings does not itself require multiple projects. A `packages` list enables native workspace discovery; patch-only settings retain single-project planning. Other configuration keys remain unsupported.

All declared and locked patch paths must reference regular files inside the captured project. Absolute paths, parent traversal, backslashes, symbolic links, missing files and patches over 4 MiB fail admission. Native pnpm verifies lock/configuration agreement and patch hashes, selects matching dependencies and applies the patch to its private installation. Oyzu does not parse diff hunks or implement its own patch engine. Modified patch bytes with a stale lock fail before project lifecycle hooks run. Source files and the content-addressed registry archives remain unchanged.

Registry inventory records continue to identify the original downloaded archives. The `oyzu.dev/pnpm.sourcePatches` extension separately records each locked patch selector, captured relative path, SHA-256 digest, size and native lock hash. The source digest binds the patch inputs, and the dependency-snapshot digest includes the patch evidence. This distinguishes source modifications from pristine upstream archive identity; it is not a complete runtime SBOM.

## Outputs and failures

Successful builds produce a snapshot-versioned native `.tgz` package under `dist/`, with its embedded package version matching the artifact version. The manifest records actual artifacts and collected test/coverage reports; native logs and the dependency snapshot accompany the bundle. Failed tests retain collected reports and block packaging. `oyzu inspect dist` checks recorded content integrity, not release eligibility or producer trust.

Dependency records use adapter layout `2`, inventory every captured archive with its name, version, source ID, size and SHA-256 digest, and record `lockfile-sha512` verification in `oyzu.dev/pnpm`. Package purpose currently means build input; runtime/test classification and registry dependency edges are not modeled. Workspace captures additionally retain native local package edges for planning and version projection. No complete runtime SBOM is claimed.

A stale lock requires regenerating and committing the lock with the intended native pnpm version before rebuilding. Digest mismatch, denied acquisition, missing captured archives or changed runtime versions fail rather than refetch during execution. Old dependency-free layout-1 captures cannot be replayed as layout 2; rebuild the toolchain image and reacquire inputs. Project sources remain unchanged by the captured build.

## Workspace builds

The [EX-020 pnpm variation](../../examples/builds/node-workspace/variants/pnpm/package.json) has a private root, a native `pnpm-workspace.yaml`, a committed v9 lock and locally linked app/shared packages. Run `oyzu build` from that root without an Oyzu configuration file. `run list` exposes build/test/lint/format-check; direct host build dispatch uses native `pnpm --recursive run build`, while direct tests use the [workspace report collector](node-workspace-tests.md).

Acquisition runs native frozen installation with scripts disabled, then asks pnpm to list its members. Local links come from the validated importer records and must resolve to captured members. The common Node workspace planner orders packages, freezes their snapshot identities and declares package-owned JUnit/LCOV. Execution projects member versions, workspace dependency selectors and matching importer specifiers in its private tree before a fresh frozen installation. Native `pnpm pack` converts workspace selectors to published snapshot dependencies. The committed source and lock remain unchanged.

Root scripts retain stage ownership. Otherwise member build scripts run in dependency order and member tests/quality use native scripts or inferred defaults. Required test/quality failures block artifact collection, with available reports retained in the failed bundle. Shared task scheduling owns overrides, prerequisites and hooks. Build formatting is read-only. Registry inputs use the captured archive-only loopback server described above; pnpm's HTTP transport to that local server does not permit external downloads.

The initial native proof covers the committed private-root, two-package fixture. Cyclic graphs, affected selection, member task groups and complete direct host build/quality composition remain functional gaps. Publishable roots, workspace registry dependencies, peer/alias combinations and framework variations still require scenario evidence. Native pnpm package layout and peer rules remain authoritative; this is not a new package resolver or a claim of complete pnpm compatibility.

`python tooling/test-native-workspace-build.py --manager pnpm` passed Windows native acquisition, fresh captured-input replay, two snapshot packages with matching local dependency versions, real JUnit/LCOV, lint/read-only formatting and failed-test reports using Node 24.14.1/pnpm 10.11.0. The same probe supports `--manager yarn`. The registered `pnpm-workspaces` group runs the actual compiled CLI in the Linux Node suite, checks every required stage, bundle inspection and source preservation, and exercises failed-test/lint gates; its result remains pending. Native adapter checks alone do not prove worker isolation.

## Compatibility and evidence

This increment supports single-project public registry packages with conventional integrity-only lock resolutions and native dependency patches, and experimental captured workspace packaging. `.npmrc`, `.pnpmfile.cjs`, configuration other than `packages` and `patchedDependencies`, overrides, explicit tarball/Git/file resolutions and private registries still require adapter work. [Yarn Classic](yarn.md) has a separate native adapter. This is an incremental implementation, not full pnpm compatibility.

The Windows native probe in `tooling/test-node-registry-acquisition.py --manager pnpm --native-cli <pnpm.cjs>` exercises real pnpm with a direct and transitive package, repeated fresh captures, offline replay of captured bytes, lifecycle isolation, stale locks, corrupted archives and rejected source hooks. This replaces the former `test-pnpm-acquisition.py --pnpm-cli` entry point. The native manager probe also covers version/engine rejection, JUnit/LCOV and repeatable packages. These host probes do not prove container isolation. The Linux compiled-CLI Node suite builds the registry fixture, validates reports and dependency records, repeats snapshot packages and exercises failed tests; revision-specific CI results remain in [implementation status](../implementation-status.md). Full captured-build execution on Windows/macOS is not established by native adapter tests.

Add `--patches` to the acquisition probe to exercise the [native patch example](../../examples/builds/node-managers/variants/pnpm-patches/package.json), both native configuration locations, changed-patch rejection before hooks and denied patch paths. The Linux compiled-CLI suite includes that fixture's snapshot artifact, four JUnit cases, coverage, source-patch evidence, repeatability and preflight failure bundle. Consult implementation status for whether that revision has passed isolated acceptance.

# pnpm builds

Oyzu recognizes a conventional Node project using pnpm and its `pnpm-lock.yaml`. A single project needs no Oyzu build configuration. The experimental adapter captures public-registry dependency archives, verifies their SHA-512 lockfile integrity, and uses native pnpm for frozen-lock validation and installation. Native pnpm retains responsibility for dependency layout and peer resolution.

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

The admitted lock format is v9 with one root importer. Every registry package in its `packages` section is captured, including transitive and optional entries, even when native pnpm does not install an entry on the selected platform. Resolution must contain a SHA-512 integrity value. Registry archive URLs derive from the locked package name/version and use the engine's scoped `npm-public` route to `https://registry.npmjs.org/`.

Acquisition disables project lifecycle scripts. Unsupported pnpm configuration and hook files are rejected before any native invocation, including the version query. Preparation verifies archive digests before handing bytes to pnpm. The snapshot contains content-addressed tarballs and a deterministic inventory; it does not retain pnpm's timestamped mutable store.

For installation, the adapter creates a fresh private store and serves the exact captured archive allowlist on an ephemeral loopback port. The isolated worker has no external network. This server never forwards requests or holds registry credentials; unknown routes fail. Native pnpm needs HTTP access to this local server, so its `--offline` switch is not used. The worker isolation and archive allowlist enforce the external-network boundary. Execution mounts no acquisition broker, rechecks archive identity, and enables native lifecycle behavior. pnpm 10's dependency-script approval rules still apply; Oyzu does not silently approve dependency scripts.

Native lock/manifest checks run with `--frozen-lockfile`. The adapter checks that the lock bytes remain unchanged and requires the same pnpm/Node versions during preparation and execution. Credentials are absent from the inventory and loopback endpoint. Managed/private registry routes remain unimplemented for this profile.

## Outputs and failures

Successful builds produce a snapshot-versioned native `.tgz` package under `dist/`, with its embedded package version matching the artifact version. The manifest records actual artifacts and collected test/coverage reports; native logs and the dependency snapshot accompany the bundle. Failed tests retain collected reports and block packaging. `oyzu inspect dist` checks recorded content integrity, not release eligibility or producer trust.

Dependency records use adapter layout `2`, inventory every captured archive with its name, version, source ID, size and SHA-256 digest, and record `lockfile-sha512` verification in `oyzu.dev/pnpm`. Package purpose currently means build input; runtime/test classification and dependency edges are not modeled. No complete runtime SBOM is claimed.

A stale lock requires regenerating and committing the lock with the intended native pnpm version before rebuilding. Digest mismatch, denied acquisition, missing captured archives or changed runtime versions fail rather than refetch during execution. Old dependency-free layout-1 captures cannot be replayed as layout 2; rebuild the toolchain image and reacquire inputs. Project sources remain unchanged by the captured build.

## Compatibility and evidence

This increment supports single-project public registry packages with conventional integrity-only lock resolutions. pnpm workspaces, `.npmrc`, `.pnpmfile.cjs`, package-level `pnpm` configuration, patches, overrides, explicit tarball/Git/file resolutions and private registries still require adapter work. [Yarn Classic](yarn.md) has a separate native adapter. This is an incremental implementation, not full pnpm compatibility.

The Windows native probe in `tooling/test-node-registry-acquisition.py --manager pnpm --native-cli <pnpm.cjs>` exercises real pnpm with a direct and transitive package, repeated fresh captures, offline replay of captured bytes, lifecycle isolation, stale locks, corrupted archives and rejected source hooks. This replaces the former `test-pnpm-acquisition.py --pnpm-cli` entry point. The native manager probe also covers version/engine rejection, JUnit/LCOV and repeatable packages. These host probes do not prove container isolation. The Linux compiled-CLI Node suite builds the registry fixture, validates reports and dependency records, repeats snapshot packages and exercises failed tests; revision-specific CI results remain in [implementation status](../implementation-status.md). Full captured-build execution on Windows/macOS is not established by native adapter tests.

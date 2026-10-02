# Yarn Classic builds

Oyzu discovers conventional single-project and workspace Yarn Classic projects from its native manifests and lockfile, without an Oyzu build file. The current adapter captures locked public-registry archives and lets Yarn install from a private offline mirror. Native Yarn owns dependency layout, peer behavior and lifecycle semantics.

## Provisioning and commands

The supported provisioned profile is Yarn Classic 1.22.22 on Node 22. Build the shared Node quality image first:

```text
docker build -t oyzu-toolchain/node:quality tooling/images/node-quality
docker build -t oyzu-toolchain/node:yarn1.22.22-node22 tooling/images/node-yarn
oyzu --root path/to/project run list
oyzu --root path/to/project build
oyzu --root path/to/project inspect dist
```

The executor requires Docker and already provisioned images. Oyzu never installs a replacement manager during a build. `--image yarn=<image>` selects a compatible provisioned image; its immutable identity is recorded. The image includes `@yarnpkg/lockfile` 1.1.0 at `/opt/oyzu-yarn/node_modules/@yarnpkg/lockfile`, `tar-stream` 3.2.1 for archive normalization, and the shared quality tools. Rebuild older custom images to include the lock parser. An exact `packageManager` declaration must agree with native Yarn's version. Incompatible Node engine requirements fail.

Native scripts appear as tasks. Conventional build/test work and read-only lint/format checks run during captured builds. Development `oyzu run` commands use host tooling; explicit formatting can modify source, while build formatting checks cannot. See [Node quality](node-quality.md) for defaults and framework configuration. The existing Node test integration collects JUnit and LCOV for native Node test scripts; supported alternative frameworks retain their own adapters.

## Capture and installation

The adapter uses Yarn's published lockfile parser for Classic v1 syntax, including grouped selectors. It requires a successfully parsed lock, current direct dependency selectors, conventional HTTPS registry archive URLs and SHA-512 integrity values. Multiple selectors for one archive share its captured identity; conflicting identities and ambiguous offline-mirror filenames fail. Every locked archive is captured, including optional and transitive entries. Git, file and other non-registry resolutions are not admitted.

Preparation uses the existing scoped broker routes for `https://registry.npmjs.org/` and `https://registry.yarnpkg.com/`, recorded as `npm-public` and `yarn-public`. Redirects remain subject to broker authorization. Source lockfiles keep their original resolved URLs and hashes. The archive inventory strips URL fragments from transport addresses and verifies the lock's SHA-512 value before native installation. Credentials never enter the inventory or mirror.

The snapshot contains content-addressed archives and inventory, not Yarn's extracted mutable cache. Each install creates a fresh private mirror/cache and a temporary Yarn configuration outside project source. Native installation consumes that mirror, with development dependencies included and no external network in the worker. Before allowing lifecycle scripts, Oyzu runs native `yarn install --offline --ignore-scripts --force` in the private tree and compares the resulting parsed lock with the captured lock. Any dependency-resolution change fails. Cosmetic serialization changes are discarded, preserving the original lock bytes. During execution, a second native `yarn install --offline --frozen-lockfile --force` runs the lifecycle without a broker. Acquisition never enables scripts. The native Yarn/Node versions must match the captured versions.

This native lock comparison is necessary because suppressing lockfile writes alone does not establish that changed `resolutions` agree with the lock. Yarn owns selective path matching, version selection and dependency layout. Registry version overrides in the native `package.json` are supported, including selecting a different transitive version while retaining a direct version:

```json
{
  "dependencies": {"is-odd": "3.0.1", "is-number": "6.0.0"},
  "resolutions": {"is-odd/is-number": "7.0.0"}
}
```

Generate and commit the corresponding lock with native Yarn before building. The [authored example](../../examples/builds/node-managers/variants/yarn-resolutions/package.json) has the complete project. Native Yarn's compatibility warnings and selective-resolution limitations still apply; Oyzu does not introduce a second override matcher. File, Git, URL and alias-protocol resolution values are currently rejected before native invocation. See Yarn's [selective resolution documentation](https://classic.yarnpkg.com/lang/en/docs/selective-version-resolutions/).

This follows Yarn's [documented offline mirror](https://classic.yarnpkg.com/blog/2016/11/24/offline-mirror/) mechanism. No package resolver or Yarn cache layout is reimplemented.

## Outputs, failures and migration

A finite `matrix.node` in `build.yaml` selects exact provisioned runtimes while retaining native Classic lock and selective-resolution semantics. Each variant has separate runtime evidence, tasks, reports and snapshot archives. See [runtime matrices](runtime-matrices.md) for provisioning, failure behavior, limits and measured verification. Modern Yarn remains unfinished; Classic workspace behavior is described below.

Successful builds emit native snapshot `.tgz` packages, normalized for repeatable archive metadata. The embedded package version matches the artifact version. `dist/manifest.json` records actual artifacts, tests and coverage, with dependency snapshots and native logs alongside it. Failed tests retain reports and block packaging. Bundle inspection checks content integrity; it does not establish release trust.

Dependency adapter layout `2` records names, versions, source IDs, SHA-256 digests and sizes. The `oyzu.dev/yarn` extension records `lockfile-sha512` integrity and all-locked-registry-tarballs inventory. Purpose is currently classified as build input; registry dependency edges and runtime/test classifications are not modeled. Workspace captures additionally retain native local package edges for build ordering and version projection. These records are not a complete runtime SBOM.

Fix stale or conflicted locks using native Yarn and commit the intended result. Corrupt archives, missing captured inputs, disallowed sources and runtime mismatches fail rather than downloading replacements during execution. Layout-1 dependency-free captures require reacquisition; they cannot be replayed as layout 2. Unsupported source configuration is rejected before any native invocation, including version queries.

## Workspace builds

The [EX-020 Yarn variation](../../examples/builds/node-workspace/variants/yarn-classic/package.json) uses a private native workspace root, a committed Classic lock and two locally linked packages. Run the commands above from that root; no Oyzu configuration is needed. `run list` includes build, test, lint and format-check. A plain `oyzu run build` dispatches native `yarn workspaces run build` when there is no root script, so Yarn requires the corresponding member scripts. The captured `oyzu build` flow also handles packages without compilation scripts.

Acquisition asks provisioned Yarn for its workspace inventory after native lock validation. Native member locations and local dependency edges become captured facts; Oyzu does not interpret Yarn's workspace globs. The planner orders members by those edges, assigns snapshot versions and declares package-owned reports. It projects versions and matching local dependency references only in the private execution tree before a fresh offline native install. Source manifests and the committed lock remain unchanged. Registry inputs still use the same captured mirror described above.

An explicit root script owns its stage once. Otherwise build scripts run in dependency order, and tests and read-only quality checks compose member scripts with inferred defaults. Shared framework adapters collect JUnit and LCOV per package. Native `yarn pack --ignore-scripts` creates each planned archive; Oyzu normalizes archive metadata and publishes the packages to `dist` only after required stages pass. Failed assertions or quality checks block artifact collection and retain available failure evidence. Hooks and overrides retain the shared task/report contracts.

Classic requires a private workspace root. Cyclic local graphs, custom registry configuration, modern Yarn, affected selection and member task groups remain unfinished. Direct host build/quality composition is less complete than captured builds; direct tests use the [workspace report integration](node-workspace-tests.md). The initial native proof covers the linked two-package fixture. Workspace combinations with external dependencies and broader framework configurations still need scenario evidence.

`python tooling/test-yarn-workspace-build.py` passed native acquisition, fresh offline linking, both snapshot packages and matching local references, real JUnit/LCOV, quality and failed-test evidence on Windows with Node 24.14.1/Yarn 1.22.22. This is an adapter probe, not a compiled-CLI isolated build. The registered `yarn-workspaces` group in the Linux Node suite exercises the compiled CLI, manifest, inspection, source preservation and failed-test/lint gates; its result remains pending. Existing standalone registry and selective-resolution probes also remain required regressions.

## Supported scope and evidence

This increment supports Classic single-project public-registry dependencies, including scoped packages and native registry version `resolutions`, and adds experimental Classic workspace capture and packaging. Modern Yarn/Berry, `.yarnrc`, `.yarnrc.yml`, `.npmrc`, custom registry layouts and private registry credentials remain required work. Native lifecycle scripts that need external downloads fail in the isolated worker. No managed registry or complete Yarn compatibility is claimed.

`tooling/test-node-registry-acquisition.py --manager yarn --native-cli <yarn.js>` exercises real Yarn with direct, transitive and scoped dependencies, repeated fresh captures, immutable offline replay, lifecycle isolation, stale manifests, corrupted bytes and unsupported configuration. Add `--resolutions` to exercise the selective override fixture and stale-override rejection with both versions already captured. Windows host probes establish native adapter behavior, not worker isolation. The Linux compiled-CLI Node suite builds both registry fixtures, checks snapshot identity, JUnit/coverage, dependency records, repeatability and failed-test retention. Current revision evidence and remaining gaps are tracked in [implementation status](../implementation-status.md). Full captured-build execution on macOS/Windows remains unproven.

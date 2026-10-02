# npm workspace builds and development tasks

This experimental integration uses npm's native workspace membership and installed dependency graph. It supports private aggregation roots and publishable root packages, with independent snapshot packages and test reports. It does not yet complete every [EX-020](../../examples/builds/node-workspace/README.md) requirement; affected selection, member task groups and duplicate target selection remain unfinished.

## Prerequisites and quick start

Provide Node/npm and install the project's native dependencies yourself. Oyzu does not install tools. Native development verification uses Node 24/npm 11 on Windows; CI provisions Node 22/npm 11 on Windows, macOS and Linux. These measured versions do not establish compatibility with every older version.

From a conventional npm workspace root containing `package.json` and its native lock:

```text
oyzu run list
oyzu run build
oyzu run test
oyzu run lint
oyzu run format-check
oyzu run format
```

No Oyzu configuration is required. `run list` is static and does not invoke Node/npm or project scripts. Explicit execution resolves installed workspace facts with npm's bundled membership library and Arborist. Missing dependencies or native tools can therefore allow listing but prevent execution. Provision dependencies with the project's normal npm workflow, then retry.

Installed workspace links must resolve to their declared member directories. Identity comparison uses native canonical paths so directory aliases do not change ownership. A link to a different directory or a copied package in place of a workspace link fails before member commands run; repair the native installation and retry.

A root package's declared workspace dependency is validated against its actual version/specification using npm's native validator. Arborist's synthetic membership edge describes a local path and is not a substitute for that dependency constraint. Invalid required declarations fail before member execution even when a workspace link exists; optional dependency handling retains native edge semantics.

Default quality tasks additionally require the selected ESLint/Prettier or Biome tools and their supporting packages. Installed project packages take precedence; a declared but missing tool fails. For a project without those tools, supply the provisioned defaults described in the [Node quality reference](node-quality.md), using `OYZU_NODE_QUALITY_HOME` for development. That directory contains the quality toolchain's `package.json` and `node_modules`. Oyzu does not fetch missing checkers.

## Task selection and ownership

| Operation | Explicit root script | Without an explicit root script |
| --- | --- | --- |
| `build` | Run it once with native npm lifecycle hooks | Run member build scripts in dependency order; report members without scripts as not requesting compilation |
| `test` | Run it once with native npm pretest/posttest hooks | Run each member's test script or detected Node/Jest/Vitest/Mocha runner; a publishable root also runs its own implicit suite |
| `lint` | Run it once | Check root-owned source; use each member's lint script or selected ESLint/Biome |
| `format-check` | Run `format-check` or `format:check` once | Check root-owned source; use each member's corresponding script or selected Prettier/Biome check |
| `format` | Run it once on explicit request | Format root-owned source; use each member's format script or selected Prettier/Biome formatting |

Native script names remain visible in task listing, including a root `format:check` alias. When that alias owns the operation, use `oyzu run format:check`; Oyzu does not add a duplicate `format-check` task. Existing TOML task replacements take precedence over inferred tasks. See [configuration](configuration.md) for settings, profiles and task override behavior.

Workspace tests resolve each member's installed framework, including hoisted dependencies. Native Jest/Vitest/Mocha entrypoints come from their package `bin` declarations. Implicit Node suites use native test filename patterns; implicit parent suites exclude nested members and engine state. Explicit scripts keep their own selection semantics, so a script can deliberately aggregate other packages. A private root without a test script is an aggregation container and has no additional implicit root suite. Native assertion failures fail the aggregate while allowing later suites to run; discovery, missing-runner or selection errors can stop the operation before later suites start. Missing Node tests currently fail explicitly instead of producing an empty successful suite. The bounded Node selection allows at most 4,096 files and 24,000 path characters.

Development test commands preserve native console output and script argument forwarding and export the test-only bundle described below. Use `oyzu build` for captured execution and snapshot artifacts. Native scripts and framework configuration execute on the host during `run test`; only static listing is nonexecuting.

Default root checks exclude workspace members. Default member checks exclude nested members, so a parent default cannot bypass a child's explicit script ownership. The current quality file scope is JavaScript/TypeScript and their supported JSX, CommonJS and ESM extensions. Dependency directories, engine state, build output and coverage are excluded; source symlinks are not followed. Native ESLint configuration and Prettier configuration/ignores are respected. This is not yet a general formatter for all repository file types.

Default checks are read-only. `format` writes files and is never selected as an implicit build check. A user-authored script can perform its own effects; its name does not establish that it is read-only. A failed member build stops dependent compilation. Quality failures continue through the other members to collect diagnostics, then fail the aggregate. Already executed script effects or explicit formatting writes are not rolled back.

Native scripts receive forwarded arguments through npm. A composed operation containing any default quality checker rejects extra arguments before invoking member operations; define a native script or explicit argv task for custom checker options. Native Biome configuration can select Biome instead of the ESLint/Prettier defaults for a member; see [Node quality](node-quality.md) for supported versions and scope. Cyclic workspace build dependencies currently fail before member build execution. Large development plans exceeding the current 20,000-byte encoded plan bound fail with a host argument-limit error.

## Direct test bundles

`oyzu run test` now writes a test-only `dist/` bundle with JUnit and LCOV for each package. No reporting section is needed. An explicit root test script owns one aggregate report pair; otherwise each member has a pair, plus an implicit root suite when the root is publishable. Named report IDs use the same native package identity as captured builds. A failing assertion fails the command but retains available sibling reports; Oyzu's success-only post hook is blocked. Run `oyzu inspect dist` to check the bundle. See [direct test evidence](direct-tests.md) for history, preserved host output and host-isolation disclosures.

This workflow uses already installed native dependencies. Native membership and installed link identities are observed only after an explicit, admitted test request; `run list` still executes no native code. The frozen report contract is built before hooks or prerequisites execute, so install the workspace dependencies before invoking tests. This does not acquire dependencies, update package versions or rewrite the lockfile. Ordinary host network access and native npm configuration remain in effect.

The same package test adapter now serves captured and host execution. Implicit suites exclude nested members, and Jest/Vitest/Mocha resolve configuration from the package being tested, including native scripted members. Native scripts retain their npm lifecycle and forwarded arguments. Framework/reporting prerequisites match the [single-package profiles](direct-tests.md#native-node-framework-prerequisites); notably Vitest needs its matching coverage provider and Mocha needs native c8.

Custom commands keep their body but must supply required reports. They receive per-package `OYZU_TEST_REPORT` and `OYZU_COVERAGE_REPORT` paths when invoked by the native workspace composition. Replacing the entire workspace task must fulfill its package report obligations or declare explicit report collection paths through the shared task contract. A marker-only command that previously returned success now fails for missing evidence; no successful test report is invented for it. Root aggregate report declarations can redirect the single pair of native destinations. Individually redirecting every native member report through configuration is not implemented.

`python tooling/test-npm-workspace-direct.py --cli <compiled-path>` copies EX-020, provisions its native local npm links, and proves direct tests, separate member reports, hooks, failed/sibling evidence, unchanged metadata and bundle inspection. The mixed-framework probe below also checks a publishable root and nested members through report bundles. CI runs these only after the CLI is compiled. Measured host results and pending CI results are recorded in [implementation status](../implementation-status.md).

## Captured builds and evidence

`oyzu build` requires Docker and a provisioned npm toolchain image; see [CLI image provisioning](README.md). It captures source and locked dependencies, resolves the native graph, and prepares snapshot versions in a private execution copy. Execution replays prepared dependencies offline. Acquisition can access configured sources; the build does not authorize arbitrary new downloads. Development `run` tasks use the host environment and normal npm configuration, so they do not provide this isolation guarantee.

```text
oyzu build
oyzu inspect dist
```

Each member declares its own npm archive with a version containing the captured source identity. A root without `private: true` additionally produces a root package; a private aggregation root does not. Local dependency references are projected to the member snapshot versions. Native npm selects package contents. Engine state is excluded; root staging rejects symlinks, inconsistent inventories and selections above 100,000 files or 1 GiB. The repository's manifests and lock are not version-edited in place. Creating an artifact, including one for a private member, does not authorize publication.

An explicit root test script owns one aggregate invocation. Otherwise member suites run individually and a publishable root also requires its own tests. Supported native Node/Jest/Vitest/Mocha integrations produce JUnit and coverage; arbitrary custom commands must satisfy their report obligations. A publishable root with no discoverable Node tests currently fails. Root framework filtering excludes members and engine state while retaining native project exclusions; Jest filtering handles canonical and aliased root paths.

Implicit member tests in captured builds use the same package-owned scope selection as development tests. Planned exclusions prevent a parent implicit suite from rediscovering its nested members; explicit scripts retain native selection. The resulting `dist/manifest.json` records retained artifacts, reports and status alongside the plan, envelope and logs. Test and quality failures block final artifact collection. Native package receipts detect archive changes between packaging and collection. Missing or invalid required reports remain failures, including under task overrides. `inspect` checks bundle integrity, not production trust or release eligibility. Earlier bundles are preserved under `.oyzu/history`.

## Verification and current limits

The native probes exercise real npm packaging, snapshot dependency references, relocated archive reproducibility, root/member reports, failures and quality ownership:

```text
python tooling/test-npm-workspace-build.py
python tooling/test-npm-workspace-tasks.py --cli target/debug/oyzu
python tooling/test-npm-workspace-tests.py --cli target/debug/oyzu
```

Use `target/debug/oyzu.exe` on Windows and provision native quality tools first. The workspace test probe also needs the native dependencies from `tooling/fixtures/jest` and `tooling/fixtures/vitest`; override their locations with `--jest-modules` and `--vitest-modules`. It verifies native Node, Jest 29 and Vitest 5, not arbitrary framework versions. [The Jest reporter probe](../../tooling/test-jest-reporting.py) additionally needs `--jest-cli` pointing to an installed native Jest entrypoint. CI builds the CLI before these host checks and runs separate compiled-CLI captured-build scenarios on Linux. Native probes do not by themselves prove sandbox behavior. Revision-specific results and outstanding cross-host verification are recorded in [implementation status](../implementation-status.md#checkpoint-38-native-workspace-test-composition).

[Mocha 11 workspace reporting](mocha.md) additionally uses native c8 10, preserves package-local reporters and produces per-package evidence for implicit and recognized scripted suites. `python tooling/test-mocha-reporting.py --cli target/debug/oyzu` provisions and exercises its native workspace fixture; the captured Node suite independently checks versioned package outputs and failed-member gating. Implicit Mocha suites use native `--ignore` options for member/engine-state exclusions; explicit native scripts retain their selection scope.

Additional test frameworks, complete framework configuration inheritance, cyclic graphs, pnpm captured workspace builds, affected selection, caching and publication remain separate work. [Native workspace test bundles](node-workspace-tests.md) now also support pnpm/Yarn direct execution. Direct test bundles contain host test evidence, not the captured build's versioned artifacts. This reference describes the npm integration, not a claim of complete builder or scenario support.

[Yarn Classic workspace builds](yarn.md#workspace-builds) now share captured artifact/report planning and stage composition, while native acquisition, version projection and packing remain manager-owned. See that reference for its narrower evidence and remaining gaps.

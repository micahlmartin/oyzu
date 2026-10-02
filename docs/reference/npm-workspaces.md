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

Default quality tasks additionally require ESLint/Prettier and their supporting packages. Installed project packages take precedence; a declared but missing tool fails. For a project without those tools, supply the provisioned defaults described in the [CLI reference](README.md), using `OYZU_NODE_QUALITY_HOME` for development. That directory contains the quality toolchain's `package.json` and `node_modules`. Oyzu does not fetch missing checkers.

## Task selection and ownership

| Operation | Explicit root script | Without an explicit root script |
| --- | --- | --- |
| `build` | Run it once with native npm lifecycle hooks | Run member build scripts in dependency order; report members without scripts as not requesting compilation |
| `test` | Run it once with native npm pretest/posttest hooks | Run each member's test script or detected Node/Jest/Vitest runner; a publishable root also runs its own implicit suite |
| `lint` | Run it once | Check root-owned source; use each member's lint script or inferred ESLint |
| `format-check` | Run `format-check` or `format:check` once | Check root-owned source; use each member's corresponding script or inferred Prettier check |
| `format` | Run it once on explicit request | Format root-owned source; use each member's format script or inferred Prettier formatting |

Native script names remain visible in task listing, including a root `format:check` alias. When that alias owns the operation, use `oyzu run format:check`; Oyzu does not add a duplicate `format-check` task. Existing TOML task replacements take precedence over inferred tasks. See [configuration](configuration.md) for settings, profiles and task override behavior.

Workspace tests resolve each member's installed framework, including hoisted dependencies. Native Jest/Vitest entrypoints come from their package `bin` declarations. Implicit Node suites use native test filename patterns; implicit parent suites exclude nested members and engine state. Explicit scripts keep their own selection semantics, so a script can deliberately aggregate other packages. A private root without a test script is an aggregation container and has no additional implicit root suite. Native assertion failures fail the aggregate while allowing later suites to run; discovery, missing-runner or selection errors can stop the operation before later suites start. Missing Node tests currently fail explicitly instead of producing an empty successful suite. The bounded Node selection allows at most 4,096 files and 24,000 path characters.

Development test commands preserve native console output and script argument forwarding. They do not yet automatically export a standalone dist bundle or enforce captured-build report obligations. Use `oyzu build` for collected JUnit/coverage evidence. Native scripts and framework configuration execute on the host during `run test`; only static listing is nonexecuting.

Default root checks exclude workspace members. Default member checks exclude nested members, so a parent default cannot bypass a child's explicit script ownership. The current quality file scope is JavaScript/TypeScript and their supported JSX, CommonJS and ESM extensions. Dependency directories, engine state, build output and coverage are excluded; source symlinks are not followed. Native ESLint configuration and Prettier configuration/ignores are respected. This is not yet a general formatter for all repository file types.

Default checks are read-only. `format` writes files and is never selected as an implicit build check. A user-authored script can perform its own effects; its name does not establish that it is read-only. A failed member build stops dependent compilation. Quality failures continue through the other members to collect diagnostics, then fail the aggregate. Already executed script effects or explicit formatting writes are not rolled back.

Native scripts receive forwarded arguments through npm. A composed operation containing any default quality checker rejects extra arguments before invoking member operations; define a native script or explicit argv task for custom checker options. Cyclic workspace build dependencies currently fail before member build execution. Unsupported inferred quality integrations, including Biome, fail explicitly rather than silently skipping checks. Large development plans exceeding the current 20,000-byte encoded plan bound fail with a host argument-limit error.

## Captured builds and evidence

`oyzu build` requires Docker and a provisioned npm toolchain image; see [CLI image provisioning](README.md). It captures source and locked dependencies, resolves the native graph, and prepares snapshot versions in a private execution copy. Execution replays prepared dependencies offline. Acquisition can access configured sources; the build does not authorize arbitrary new downloads. Development `run` tasks use the host environment and normal npm configuration, so they do not provide this isolation guarantee.

```text
oyzu build
oyzu inspect dist
```

Each member declares its own npm archive with a version containing the captured source identity. A root without `private: true` additionally produces a root package; a private aggregation root does not. Local dependency references are projected to the member snapshot versions. Native npm selects package contents. Engine state is excluded; root staging rejects symlinks, inconsistent inventories and selections above 100,000 files or 1 GiB. The repository's manifests and lock are not version-edited in place. Creating an artifact, including one for a private member, does not authorize publication.

An explicit root test script owns one aggregate invocation. Otherwise member suites run individually and a publishable root also requires its own tests. Supported native Node/Jest/Vitest integrations produce JUnit and coverage; arbitrary custom commands must satisfy their report obligations. A publishable root with no discoverable Node tests currently fails. Root framework filtering excludes members and engine state while retaining native project exclusions; Jest filtering handles canonical and aliased root paths.

Implicit member tests in captured builds use the same package-owned scope selection as development tests. Planned exclusions prevent a parent implicit suite from rediscovering its nested members; explicit scripts retain native selection. The resulting `dist/manifest.json` records retained artifacts, reports and status alongside the plan, envelope and logs. Test and quality failures block final artifact collection. Native package receipts detect archive changes between packaging and collection. Missing or invalid required reports remain failures, including under task overrides. `inspect` checks bundle integrity, not production trust or release eligibility. Earlier bundles are preserved under `.oyzu/history`.

## Verification and current limits

The native probes exercise real npm packaging, snapshot dependency references, relocated archive reproducibility, root/member reports, failures and quality ownership:

```text
python tooling/test-npm-workspace-build.py
python tooling/test-npm-workspace-tasks.py --cli target/debug/oyzu
python tooling/test-npm-workspace-tests.py --cli target/debug/oyzu
```

Use `target/debug/oyzu.exe` on Windows and provision native quality tools first. The workspace test probe also needs the native dependencies from `tooling/fixtures/jest` and `tooling/fixtures/vitest`; override their locations with `--jest-modules` and `--vitest-modules`. It verifies native Node, Jest 29 and Vitest 5, not arbitrary framework versions. [The Jest reporter probe](../../tooling/test-jest-reporting.py) additionally needs `--jest-cli` pointing to an installed native Jest entrypoint. CI builds the CLI before these host checks and runs separate compiled-CLI captured-build scenarios on Linux. Native probes do not by themselves prove sandbox behavior. Revision-specific results and outstanding cross-host verification are recorded in [implementation status](../implementation-status.md#checkpoint-38-native-workspace-test-composition).

Additional test frameworks, complete framework configuration inheritance, standalone task report collection, cyclic graphs, pnpm/Yarn workspaces, affected selection, caching and publication remain separate work. Development tasks do not produce the captured build's versioned artifacts or dist evidence. This reference describes the npm integration, not a claim of complete builder or scenario support.

# Node lint and formatting

Node builders expose implicit `lint`, `format-check` and explicitly mutating `format` tasks. This experimental integration serves single packages using npm, pnpm or Yarn and npm workspace composition. It does not make all package-manager workspace forms supported. Native scripts and TOML task replacements retain precedence; no additional Oyzu quality section is required.

## Selection and usage

Separate static detectors select the linter and formatter. ESLint and Prettier are defaults; their native configuration provides stronger evidence. A `biome.json` or `biome.jsonc` selects Biome for both roles. Conflicting native evidence for the same role fails discovery rather than choosing by registration order. Discovery reads configuration evidence without executing JavaScript configuration or launching tools.

From the package or npm workspace root:

```text
oyzu run list
oyzu run lint
oyzu run format-check
oyzu run format
```

An explicit `format:check` script retains that task name. Build planning treats it as the read-only formatting operation. Implicit checks reject extra arguments before executing a checker; use a native package script or explicit argv task for custom flags. This prevents forwarded arguments from changing a lint/check request into a formatting write. Custom scripts have their own effects; Oyzu does not infer that an arbitrary script is read-only from its name.

`oyzu build` includes lint and read-only formatting as gates before final artifact collection. A native failure remains a failed action and blocks collected artifacts. `format` is never automatically selected as a build stage. Explicit formatting can leave partial changes when a later file or package fails; it is not a transactional operation.

## Tools and provisioning

| Integration | Provisioned default | Current adapter behavior |
| --- | --- | --- |
| ESLint | 10.11.0, with pinned recommended JS/TS support | Native configuration when present; recommended JS/TS rules otherwise; no automatic fixes |
| Prettier | 3.9.9 | Native configuration, EditorConfig and ignore files; check or explicit formatting |
| Biome | 2.5.15 | Biome 2 native lint and format commands; native configuration and ignores; only explicit format adds `--write` |

Project-installed packages take precedence, including native hoisted resolution. A tool declared in the current package but missing from its installed dependencies fails instead of silently using a fallback version. Other unavailable tools can use an explicitly provisioned directory identified by `OYZU_NODE_QUALITY_HOME`. That directory must contain the toolchain `package.json` and installed `node_modules`; Oyzu does not download checkers during a build or development task.

The authoritative default versions and native dependency lock are in [the quality toolchain](../../tooling/images/node-quality/package.json). To provision the development defaults yourself:

```text
npm ci --ignore-scripts --no-audit --no-fund --prefix tooling/images/node-quality
```

Set `OYZU_NODE_QUALITY_HOME` to that directory's absolute path when invoking development tasks outside the repository. For captured builds, first build the common image with `docker build -t oyzu-toolchain/node:quality tooling/images/node-quality`, then the relevant npm/pnpm/Yarn image as described in the [CLI reference](README.md). The image identity binds the provisioned defaults. Biome's optional platform binary packages must be present; missing binaries fail without an installation fallback. Its upstream wrapper selects the native binary; Oyzu does not copy that platform-selection implementation.

Local verification uses Windows with provisioned Node 24/npm 11; CI uses Node 22 and runs CLI/native task checks on Windows, macOS and Linux. Captured build scenarios run separately on Linux. The listed defaults are exact tested versions, not proof that every version in their major range works. Legacy ESLint configuration needs an installed compatible ESLint 8/9 or migration to flat configuration. The Biome adapter rejects non-2.x packages; other 2.x versions still require native compatibility verification.

## Scope, configuration and effects

Implicit quality checks currently select JS/JSX/TS/TSX and their CommonJS/ESM extensions. They do not format JSON, CSS or other non-code files, even where the native tool supports them. Captured builds project versions into package metadata, so original-metadata formatting needs a distinct source-validation contract; it is not silently inferred from rewritten build inputs. Broader formatting is available through an explicitly authored native script, and automatic non-code coverage remains work.

Source traversal excludes dependency directories, `.git`, `.oyzu`, `.oyzu-build`, `dist`, `build` and coverage output, and never follows source symlinks. It is bounded to 100,000 source candidates. npm workspace root defaults exclude member roots, and member defaults exclude nested members; scripts retain their own scope. See [npm workspaces](npm-workspaces.md) for composition and failure behavior.

Detected [Mocha](mocha.md) projects receive Mocha globals in fallback ESLint rules for conventional `test`/`tests` directories and test/spec files. Custom test locations can declare native lint configuration. The common toolchain also provisions c8 for captured Mocha coverage; that integration belongs to the reporting adapter and does not run during lint or formatting.

ESLint preserves native configuration and uses Node globals for fallback rules, plus Jest globals for inferred test files. Recognized [Vite applications](node-applications.md) also receive browser globals, and their inferred native output directory is excluded from implicit checks. Prettier respects native options and ignores. Biome receives explicit file arguments and applies its native configuration/ignore rules; it does not use stdin mode, which has different ignore semantics. Biome invocations are split into bounded argument batches for host compatibility, retaining failure if any batch fails. Unsupported/ignored files can yield no applicable check; native output and actual exit status are retained, not synthesized lint results.

Native configurations and plugins can execute their own code where the native tool supports that. Development tasks execute on the host with the task environment. Captured actions use the prepared offline execution environment, and these quality adapters do not introduce network acquisition or credential delivery. Global inherited configuration, every plugin form and complete cross-manager configuration inheritance still require further qualification.

For configured Vite applications, implicit checks exclude the output location recorded by the captured native build. When that record does not yet exist, the explicit quality task resolves Vite configuration in production build mode to obtain the exclusion, preserving a recognized native `--outDir` override. That resolution may execute native configuration code, just as native checker configurations can; it never happens during discovery or task listing. Invalid output metadata or an uncontained configured output fails the check instead of silently expanding the source scope.

## Verification and troubleshooting

```text
python tooling/test-node-quality.py --cli target/debug/oyzu
python tooling/test-biome-quality.py --cli target/debug/oyzu
python tooling/test-npm-workspace-tasks.py --cli target/debug/oyzu
```

Use `target/debug/oyzu.exe` on Windows. The Biome probe checks native JSONC configuration, real lint/format failures, unchanged input bytes during checks, explicit formatting, native ignores, private-state exclusions, argument guards, batched failure retention and installed-tool precedence. Workspace checks mix inferred tools with scripts. Native probe success alone does not establish sandbox execution; the separate CI build cases verify snapshot packages, reports and failed gates through the compiled CLI.

For a missing declared tool, install the project's native dependencies and retry. For a missing default, provision the toolchain explicitly. For conflicting evidence, resolve the competing native configurations; task overrides do not bypass ambiguous static discovery. Native diagnostics remain authoritative for invalid configuration and failed rules. Revision-specific results and remaining gaps are recorded in [implementation status](../implementation-status.md).

Native integration reference: [Biome CLI](https://biomejs.dev/reference/cli/).

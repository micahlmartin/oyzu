# Native Node workspace test bundles

Status: experimental. `oyzu run test` collects package-owned JUnit and LCOV for npm, pnpm 10 and Yarn Classic workspaces. Native managers determine workspace membership; Oyzu infers each package's test runner and uses the shared report collector. This direct host flow requires no build YAML or reporting section.

```text
oyzu run list
oyzu run test
oyzu inspect dist
```

## Tools and discovery

Install the project's dependencies and provision Node and its native package manager before running tests. Oyzu does not acquire tools or dependencies during this operation. Measured local versions are Node 24.14.1, npm 11.11.0, pnpm 10.11.0 and Yarn 1.22.22 on Windows. CI provisions Node 22 and those manager versions after compiling the CLI. New pnpm/Yarn cross-platform acceptance remains pending.

Static listing reads native markers and scripts without executing a package manager. `package.json` workspaces identify npm/Yarn workspaces; `pnpm-workspace.yaml` identifies pnpm even before a lock exists. Native lockfiles and `packageManager` declarations still participate in the normal manager conflict rules. Actual membership is observed only after an explicit admitted test request: npm uses its installed graph adapter, pnpm uses its native recursive JSON list, and Yarn Classic uses its native workspace-info output. Oyzu does not implement a competing workspace glob parser. See [pnpm list](https://pnpm.io/10.x/cli/list) and [Yarn Classic workspace commands](https://classic.yarnpkg.com/lang/en/docs/cli/workspaces/).

pnpm/Yarn must expose their provisioned JavaScript entrypoint on PATH through a normal package installation or native executable link. This supports package-local `.bin` directories and ordinary global installations on Windows and Unix without spawning a `.cmd` shim through a shell. Corepack-based acquisition and standalone bundled pnpm executables are not used by this adapter. pnpm automatic manager-version management is disabled; Yarn uses offline mode and ignores `yarn-path` redirection. Other native project settings still apply. Missing prerequisites or an unsupported manager major version fail explicitly.

## Task and report ownership

Without a root test script, listing shows the native recursive test command for the selected manager. At execution, Oyzu observes membership, freezes package scopes and report obligations, then runs each member's native script or inferred Node/Jest/Vitest/Mocha suite. An explicit root test script instead owns one aggregate report pair and runs once. A publishable root without a script also owns an implicit suite where the native manager permits that root layout. Yarn Classic requires a private workspace root.

Native package scripts keep their manager's argument and lifecycle behavior. Inferred suites exclude nested members; explicit scripts retain their own selection semantics. Framework dependencies and reporters must already be installed; see [native reporting prerequisites](direct-tests.md#native-node-framework-prerequisites). pnpm/Yarn native Node suites and root aggregation are verified locally; their broader mixed-framework combinations remain to be qualified. The existing npm mixed-framework probe exercises the shared execution layer separately.

Each package's current JUnit and LCOV files enter a test-only `dist/` bundle with plan, logs and manifest summaries. Package names provide stable report identities. A failing assertion fails the command while allowing later suites to run and retaining available reports. Missing native tools, malformed metadata or unresolved framework selection may stop execution before any suite starts. Missing or invalid required reports remain errors, including under task overrides. No success is manufactured for a command that emits no evidence.

Oyzu pre/post hooks use the common task engine. A failed test blocks its success-only post hook. Exact implicit commands receive report instrumentation; arbitrary TOML replacements retain their bodies and must fulfill the required report contract. Member commands receive `OYZU_TEST_REPORT` and `OYZU_COVERAGE_REPORT`. Replacing a whole workspace task must supply its named obligations or explicit report collection paths. Forwarded arguments reach each selected native test command; a root script receives them once.

## Effects, limitations and verification

Direct tests do not rewrite package versions or locks, build snapshot archives, or run unrelated quality gates. Native test scripts may write host files and access the host environment/network. Offline manager flags restrict native acquisition behavior, not arbitrary script effects. Installed dependencies are consumed as they exist; this is not frozen dependency provenance or publication authority. See [direct test evidence](direct-tests.md) for history, existing output preservation and failed-bundle inspection.

Only npm currently has captured workspace packaging and full implicit member build/quality composition. pnpm/Yarn captured workspace artifacts, dependency preparation and member build/quality defaults remain functional gaps. A pnpm YAML workspace is now identified as a workspace by the build planner, preventing accidental root-only packaging. Yarn modern releases and all workspace configurations are not covered by this increment. [npm workspace builds](npm-workspaces.md) documents the separately implemented captured path.

Run the compiled-CLI probe after provisioning native fixture packages in `tooling/images/node-pnpm` and `tooling/images/node-yarn`, plus `tooling/design-requirements.txt`:

```text
python tooling/test-npm-workspace-direct.py --cli <compiled-path> --manager npm
python tooling/test-npm-workspace-direct.py --cli <compiled-path> --manager pnpm
python tooling/test-npm-workspace-direct.py --cli <compiled-path> --manager yarn
```

The probe adapts EX-020's native manifests for each manager and explicitly provisions its local workspace links before product execution. It verifies inferred and scripted member tests, hooks, failed/sibling evidence, source/lock preservation, root aggregation and inspection. CI runs all three after CLI compilation. [Implementation status](../implementation-status.md) records revision-specific results without treating a host test bundle as captured artifact acceptance.

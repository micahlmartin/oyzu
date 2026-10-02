# Direct test reports

`oyzu run test` produces a test-only `dist/` bundle for single-package Node projects and npm workspaces using `node:test`, Jest, Vitest or Mocha, Python projects using pytest through pip, uv or Poetry, Go modules/workspaces using native testing, Helm charts using local validation and optional helm-unittest suites, Rust apps/workspaces using nextest and Cargo doctests, supported Ant assertion/JUnit targets, native Maven reactors, and Gradle multi-project/composite builds. Native tools, dependencies and any required reporters must already be installed. There is no Docker, desktop, sign-in or tool-installation prerequisite for this host workflow. See [Python test reporting](python-testing.md), [Go testing](go.md#direct-test-evidence) and [Helm testing](helm.md) for native prerequisites and coverage scope; the examples below use Node.

```text
oyzu run list
oyzu run test
oyzu inspect dist
oyzu run test -- --test-name-pattern=greeting
```

The builder recognizes the conventional runner or a supported exact test command and supplies native JUnit and applicable application coverage reporting. Helm records coverage as inapplicable, with one local-validation report and a separate native unittest report when detected. Host Helm tests require prepared chart dependencies and Python on PATH; they do not install tools or download dependencies. Existing npm pretest/posttest lifecycle behavior stays native. Qualified names such as `api:test` use that target's configuration and working directory; Oyzu pre_/post_ hooks and declared prerequisites follow the task engine's ordering. Forwarded arguments apply only to the requested task.

## Outputs and failures

The invocation uses the same report binding, validation, collection and bundle transaction as captured builds. `dist/manifest.json` links current JUnit and coverage reports (LCOV for Node, Cobertura for Python, native statement profiles for Go), their summaries, the recorded plan, execution envelope and logs. It records no newly built application artifacts and does not run packaging, lint or formatting just because tests were requested. Use `oyzu build` for builder stages and snapshot artifacts.

Native test failures, missing required reports, malformed reports and an unmet configured coverage minimum fail the command. Valid failed-test reports and bounded malformed report bytes are retained when available. Collection waits for the post-hook so that a report-transforming hook can finish. A failing pre-hook blocks its test; a failing test skips its success-only post-hook; a failing post-hook retains the test's available reports. A launch failure produces failed action evidence rather than success. Unexecuted actions are marked blocked.

Default reports use a fresh private directory for every invocation. Custom commands are not parsed as arbitrary shell programs or replaced with guessed native commands. They must fulfill the same required report obligations; hooks and commands receive `OYZU_TEST_REPORT` and `OYZU_COVERAGE_REPORT` for concrete assigned paths. A literal declaration can redirect a supported exact command:

```toml
[tasks."project:test"]
argv = ["node", "--test"]
reports = [{ kind = "test", format = "junit", path = "reports/junit.xml" }]
```

The path is relative to the task's working directory. Oyzu creates missing parent directories for literal paths. A report declaration does not remove the other required report kinds. Explicit report paths or globs must have no existing matching output before the invocation; preserve or remove earlier files before rerunning. Oyzu fails rather than treating stale evidence as current. For a glob, the command remains responsible for producing matching files and directories; no single output path environment variable is inferred for multiple files.

## Existing application output

Host tasks often build into `dist/` before running tests. Direct tests preserve those files alongside their new evidence, recording paths, sizes and digests under `extensions.oyzu.dev/host-outputs`. Previous artifact bytes are preserved as ordinary host output; they are not listed as artifacts produced by this test invocation. Inspection verifies the preserved inventory. Earlier Oyzu reports, evidence and logs stay in history rather than becoming current test results.

The previous directory moves to `.oyzu/history/<new-run-id>` through the shared transaction. Native output must use portable regular files/directories; links, reparse points and collisions with new evidence fail. `manifest.json`, `plan.json`, `envelope.json` and `logs` are reserved bundle paths. An application-owned manifest or other reserved-path collision needs to be preserved or relocated before this workflow can finalize. Failures during preflight or output preservation can leave the earlier bundle in place; check the command outcome and run identity instead of assuming any existing `dist` belongs to the latest attempt. See [bundle retention and recovery](build-bundles.md).

## Evidence scope and compatibility

The plan, envelope and manifest disclose `oyzu.dev/invocation` with `kind: test`, `execution: host`, `isolation: none`, `source: observed-before-run` and `toolIdentity: unverified`. Source identity is observed before executing against the live checkout; dependencies, ambient tools and subsequent source mutations are not captured. Actions declare `network: host`, are not cacheable, and disclose that sandbox resource limits are not enforced. Environment values in plan records are redacted. Native output/logs remain the output of the invoked programs; they are not a general secret-redaction service.

CI detection gives `ci-unverified`, not a trusted producer identity. Effective configuration identity and available management metadata are recorded, but this test invocation does not satisfy unrelated full-build checks or grant publication/signing authority. Inspection checks integrity and agreement of invocation records, not authenticity. This is local development evidence, not hermetic or production provenance.

Integrated profiles are single-package Node tests and npm workspaces (`node:test`, Jest, Vitest and Mocha), Python's pytest default, native Go module/workspace tests and Helm charts. Rust app/workspace native acceptance passed on all three CI hosts as described below. [Ant tests](ant.md) and [Maven reactor tests](maven.md#direct-test-bundles) connect their native test and JaCoCo adapters to this collector. [Gradle composite tests](gradle.md#direct-test-bundles) also supply named native reports. pnpm/Yarn workspaces and Docker still lack this test-only bundle integration. A supported test depending on an unsupported test profile fails before execution rather than claiming complete evidence for both. These are implementation gaps; the intended common builder reporting contract remains broader.

The native probe is `python tooling/test-direct-test-bundles.py --cli <compiled-oyzu-path>`, with dependencies from `tooling/design-requirements.txt`. It checks real native tests, application coverage, lifecycle behavior, hooks, failed/missing/malformed evidence, redirected/stale reports, launch failures, native output preservation, history and inspection tampering. CI runs it after CLI compilation on Windows, macOS and Linux. Revision-specific results and remaining gaps are recorded in [implementation status](../implementation-status.md); CI wiring alone does not prove all hosts passed.

`python tooling/test-helm-direct.py --cli <compiled-oyzu-path> --helm <helm-path>` exercises application and root library charts, qualified targets/hooks, native unittest, failed reports, coverage inapplicability and bundle inspection. Helm 3.22.0, helm-unittest 1.2.0 and Python 3.11+ must be provisioned. The shared `TaskPlan` carries coverage applicability into target records, including after post-hooks; a positive coverage minimum cannot be satisfied by a chart without a coverage report. The Linux Helm suite runs this probe after downloading the compiled CLI. Windows native results and pending Linux execution are recorded in implementation status; macOS native execution is not yet qualified.

## Native Node framework prerequisites

Jest 29.7.0, Vitest 5.0.3 with matching `@vitest/coverage-v8` 5.0.3, and Mocha 11.8.0 with c8 10.1.3 are exercised by the direct framework probe. Provision project dependencies before `oyzu run test`. Mocha requires project-installed c8 10.x or explicitly provisioned fallback tools via `OYZU_NODE_REPORTING_HOME`; see [Mocha reporting](mocha.md). Tests do not install missing reporters. Vitest's declared coverage provider must match its version; other framework major versions are not qualified by these checks.

No test section is needed. Detection selects the implicit runner; exact native scripts (`jest`, `jest --ci`, `vitest`, `vitest run`, `mocha`) keep npm pretest/posttest behavior while the existing framework adapter adds JUnit and LCOV. Native configuration remains authoritative for test selection, reporters and coverage scope. Both implicit and scripted tests use the common bundle collector, with native failures retained in the manifest. Recognized command overrides retain report obligations; arbitrary script bodies must supply their assigned reports themselves.

The framework wrappers resolve npm to its provisioned JavaScript entrypoint, preserving argument arrays and avoiding shell-dependent `.cmd` invocation on Windows. They retain native npm configuration. The three-platform CI probe covers implicit runners and npm scripts; pnpm/Yarn scripted host combinations are not yet qualified by this probe. Host tests retain ambient network access as disclosed in the invocation; they do not establish hermetic build evidence.

Run `python tooling/test-node-framework-direct.py --cli <compiled-path>` after `npm ci` in `tooling/fixtures/jest`, `tooling/fixtures/vitest`, `tooling/fixtures/mocha` and `tooling/images/node-quality`, with `tooling/design-requirements.txt` installed. It invokes the real CLI and native frameworks, validates JUnit/LCOV bundles, checks native/Oyzu hook ordering and failed assertions, and calls `oyzu inspect dist`. Fixture copies preserve npm's executable symlinks: dereferencing Unix `.bin` entries breaks Vitest's relative imports before the native test command can run. The initial Linux/macOS probe exposed that harness error. The corrected probe passed on all three hosts at bfa2b7c in run 37049720609. npm workspace report ownership and native prerequisites are described in [npm workspaces](npm-workspaces.md); pnpm/Yarn workspace integration remains unfinished.

Rust app/workspace direct tests use the same collector with nextest JUnit, Cobertura and doctest invocation reports. The direct reporting step passed on Windows MSVC, Linux and macOS at bfa2b7c. The local Windows GNU toolchain lacks the required profiler runtime. See [Rust prerequisites and current verification](rust.md#direct-test-evidence) for measured scope and unsupported combinations.

# Mocha tests and coverage

The experimental Node builder detects Mocha from an exact `mocha` test script, native `.mocharc` configuration, a `mocha` section in package.json, or a declared dependency. Configuration files are observed without executing them during discovery. Declared commands have stronger evidence than configuration or dependencies; equally strong conflicting framework evidence fails selection.

## Native project and prerequisites

Keep the existing project and native configuration. No Oyzu test/report section is required:

```json
{
  "scripts": {"test": "mocha"},
  "devDependencies": {"mocha": "11.8.0"}
}
```

With no test script, a detected Mocha project exposes `node node_modules/mocha/bin/mocha.js` as its implicit test task. `oyzu run list` shows it and `oyzu run test` executes the native development task. Single-package direct tests also collect JUnit and LCOV in a test-only `dist/` bundle; see [direct test evidence](direct-tests.md). Direct workspace report bundles remain unfinished.

Captured builds require a declared, captured Mocha dependency. Mocha 11 is the qualified API family, tested at 11.8.0. The common Node toolchain includes c8 10.1.3 for coverage. A project-installed c8 takes precedence and must be version 10.x; a declared but missing c8 fails instead of silently using the fallback. Rebuild `tooling/images/node-quality` and its npm/pnpm/Yarn descendant images after updating the toolchain lockfile. No build action downloads reporting tools. Host adapter probes and single-package direct tests use `OYZU_NODE_REPORTING_HOME` to locate explicitly provisioned fallback tooling when the project does not supply c8. This is a prerequisite for automatic direct coverage; a missing reporter fails explicitly.

## Reports and failure gates

Captured actions and single-package direct tests run the native test command under c8. Recognized scripts retain their package manager's lifecycle behavior. Mocha loads native test configuration and plugins in the test action. The reporter adapter composes the configured native reporter with Mocha's own XUnit reporter, preserving native reporter options without implementing an XML serializer or test counters. This qualified adapter uses Mocha 11's native option loader to recover reporter selection; configuration evaluation and lifecycle counts have regression checks.

c8 retains native include/exclude scope, source-map handling and thresholds from package metadata or configuration files. It writes required LCOV into a fresh private directory; configured alternate coverage formats are not additionally exported. Without a native `all` setting, c8's default denominator covers loaded source files, not necessarily every repository file. Configure broader scope natively when needed. See [c8's options](https://github.com/bcoe/c8/blob/main/README.md) and [Mocha's XUnit reporter](https://mochajs.org/reporters/xunit/).

The engine collects JUnit and LCOV under `dist/<target>/reports/` and records paths, digests and summaries in the manifest. Failed assertions retain available reports and block packaging. A failed native coverage threshold also fails the action even when all tests passed. Configuration/startup failures cannot reuse an earlier invocation's reports. Correct the native failure and rerun; action logs and diagnostics explain missing evidence. Successful builds retain the ordinary versioned Node package artifact.

Build, test, lint and read-only formatting remain ordinary builder stages. ESLint fallback rules recognize Mocha globals in conventional test directories and test/spec files; native lint configuration remains authoritative.

## Compatibility and verification

Automatic captured adaptation recognizes bare `mocha`, its known direct Node entrypoint and equivalent exact task overrides. Arbitrary shell programs and scripts with additional flags retain their bodies and must satisfy their report contract without guessed rewriting. Native configuration is the supported customization path for recognized invocations. Watch mode is disabled. Parallel Mocha reporting requires further integration and fails explicitly where detected. Browser runners and every custom reporter form are not qualified by this initial profile.

npm workspaces compose implicit and recognized scripted Mocha suites using the same reporting adapter. Each member resolves its native runner, configuration and c8 coverage scope from its own directory, including hoisted installations. A publishable root without a test script has a separate implicit suite that excludes workspace members and engine state through native Mocha `--ignore` options. Existing native exclusions remain in force. Explicit scripts retain their native selection, including `--file` entries that Mocha does not filter through `--ignore`. An explicit root test script owns a single aggregate report pair. Otherwise each package receives its own JUnit/LCOV pair; assertion failures retain sibling results and block artifact collection. See [npm workspace ownership](npm-workspaces.md) and [Mocha 11 file selection](https://v11.mochajs.org/running/cli/).

Native configuration is executable project code. Development tasks run on the host; captured tests use the prepared offline sandbox. Reporting receives no upstream credential or acquisition channel. Report integrity does not establish release eligibility.

`tooling/test-mocha-reporting.py --cli <compiled-path>` explicitly provisions fixture dependencies and checks discovery, no-script tasks, reporter preservation, configuration/lifecycle counts, escaped/skipped/failed tests, LCOV, coverage thresholds and startup failure. It also exercises the [workspace fixture](../../tooling/fixtures/mocha-workspace/) through compiled development tasks and the native reporting adapter, including package scope and root-script precedence. CI runs it after CLI compilation on Windows, macOS and Linux. The Node captured-build suite separately builds the single-package and workspace fixtures, verifies snapshot package versions/reports, and rejects assertion/threshold failures. Consult [implementation status](../implementation-status.md) for completed checks; registration is not proof of passing execution.

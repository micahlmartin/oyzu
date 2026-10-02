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

With no test script, a detected Mocha project exposes `node node_modules/mocha/bin/mocha.js` as its implicit test task. `oyzu run list` shows it and `oyzu run test` executes the native development task. Direct task report bundles remain unfinished; automatic bundle behavior below applies to `oyzu build`.

Captured builds require a declared, captured Mocha dependency. Mocha 11 is the qualified API family, tested at 11.8.0. The common Node toolchain includes c8 10.1.3 for coverage. A project-installed c8 takes precedence and must be version 10.x; a declared but missing c8 fails instead of silently using the fallback. Rebuild `tooling/images/node-quality` and its npm/pnpm/Yarn descendant images after updating the toolchain lockfile. No build action downloads reporting tools. Host adapter probes use `OYZU_NODE_REPORTING_HOME` to locate explicitly provisioned fallback tooling; ordinary native development tests do not need this adapter setting.

## Reports and failure gates

The captured action runs the native test command under c8. Recognized scripts retain their package manager's lifecycle behavior. Mocha loads native test configuration and plugins in the test action. The reporter adapter composes the configured native reporter with Mocha's own XUnit reporter, preserving native reporter options without implementing an XML serializer or test counters. This qualified adapter uses Mocha 11's native option loader to recover reporter selection; configuration evaluation and lifecycle counts have regression checks.

c8 retains native include/exclude scope, source-map handling and thresholds from package metadata or configuration files. It writes required LCOV into a fresh private directory; configured alternate coverage formats are not additionally exported. Without a native `all` setting, c8's default denominator covers loaded source files, not necessarily every repository file. Configure broader scope natively when needed. See [c8's options](https://github.com/bcoe/c8/blob/main/README.md) and [Mocha's XUnit reporter](https://mochajs.org/reporters/xunit/).

The engine collects JUnit and LCOV under `dist/<target>/reports/` and records paths, digests and summaries in the manifest. Failed assertions retain available reports and block packaging. A failed native coverage threshold also fails the action even when all tests passed. Configuration/startup failures cannot reuse an earlier invocation's reports. Correct the native failure and rerun; action logs and diagnostics explain missing evidence. Successful builds retain the ordinary versioned Node package artifact.

Build, test, lint and read-only formatting remain ordinary builder stages. ESLint fallback rules recognize Mocha globals in conventional test directories and test/spec files; native lint configuration remains authoritative.

## Compatibility and verification

Automatic captured adaptation recognizes bare `mocha`, its known direct Node entrypoint and equivalent exact task overrides. Arbitrary shell programs and scripts with additional flags retain their bodies and must satisfy their report contract without guessed rewriting. Native configuration is the supported customization path for recognized invocations. Watch mode is disabled. Parallel Mocha reporting and npm workspace Mocha composition require further integration and fail explicitly where detected. Browser runners and every custom reporter form are not qualified by this initial profile.

Native configuration is executable project code. Development tasks run on the host; captured tests use the prepared offline sandbox. Reporting receives no upstream credential or acquisition channel. Report integrity does not establish release eligibility.

`tooling/test-mocha-reporting.py --cli <compiled-path>` explicitly provisions fixture dependencies and checks discovery, no-script tasks, reporter preservation, configuration/lifecycle counts, escaped/skipped/failed tests, LCOV, coverage thresholds and startup failure. CI runs it after CLI compilation on Windows, macOS and Linux. The Node captured-build suite separately builds the [native fixture](../../tooling/fixtures/mocha/), verifies snapshot package versions/reports, and rejects assertion/threshold failures. Consult [implementation status](../implementation-status.md) for completed checks; registration is not proof of passing execution.

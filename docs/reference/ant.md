# Ant test reporting

Ant projects with a conventional `test` target can run `oyzu run test` to produce native test and coverage evidence in a test-only `dist/` bundle. The direct adapter reuses the captured builder's Ant listener, JUnit formatter and JaCoCo integration. It does not create a new test framework or require an Oyzu configuration file for the conventional case.

```text
oyzu run list
oyzu run test
oyzu inspect dist
```

For an existing custom target, a task replacement selects it:

```toml
[tasks.test]
argv = ["ant", "verify-contract"]
```

One literal `ant <target>` command is instrumented, including the equivalent literal shell spelling used in the custom example. Compound shell commands and arbitrary replacements retain their bodies and must produce the required JUnit and JaCoCo reports themselves, using the assigned `OYZU_TEST_REPORT` and `OYZU_COVERAGE_REPORT` paths. Extra forwarded arguments to the automatic adapter are currently unsupported and fail explicitly. Target-qualified names and pre/post hooks use the common task engine. A failed test skips its success-only post hook.

Discovery always supplies an implicit builder-owned `test` slot. A root `[tasks.test]` replacement in a single-target project selects a custom Ant target while retaining report obligations through the shared task owner resolution. Without a native `test` target or a replacement, execution fails with Ant's missing-target error; discovery does not fabricate a test suite.

## Native prerequisites

Provision a JDK with `java` and `javac`, Python, Ant and JaCoCo before execution. Set `JAVA_HOME` to the JDK or put both executables on `PATH`. Set `ANT_HOME` to the Ant distribution and `JACOCO_HOME` to a directory containing `jacocoant.jar` and `jacocoagent.jar`. JUnit projects also need their normal test libraries; the CI fixture puts JUnit and Hamcrest in Ant's `lib` directory. Native libraries are loaded from that directory. The adapter compiles its owned Java integration into a temporary directory and launches Ant using the host's classpath separator. It does not install tools or download dependencies.

The CI provisioner supplies checksum-pinned Ant 1.10.18, JaCoCo 0.8.13, JUnit 4.13.2 and Hamcrest 1.3:

```text
python tooling/provision-ant-reporting.py --destination /absolute/path/ant-reporting
python tooling/test-ant-direct.py --cli /absolute/path/oyzu --tools /absolute/path/ant-reporting
```

The first command is explicit online test setup, outside product execution. The second additionally needs `tooling/design-requirements.txt`. CI uses JDK 17 and runs the probe after compiling the CLI on Windows, Linux and macOS; registration alone is not verified platform support.

## Reports and native behavior

The manifest links JUnit outcomes and JaCoCo application coverage. Native JUnit suites retain individual tests and skipped cases. Conventional forked Java assertion programs produce one case per invocation. These programs require `fork="true"` and `failonerror="true"`. Coverage follows the existing adapter's conventional `src` ownership and observed `javac` output; test classes are excluded. Builds without line debug data retain the available instruction counters rather than inventing line coverage. Arbitrary source layouts, alternate task implementations and JUnit Platform integration require further work or explicit reports.

Direct tests keep native project version properties, execute the target's compilation prerequisites, and can update native build output. They do not package snapshot artifacts or run lint/format checks. `oyzu build` remains the captured path that projects snapshot versions, runs quality gates and collects JAR artifacts. Both paths use the same native reporting implementation; captured execution retains its fixed tool locations and isolation.

Missing tools, compilation errors, native test failures, missing or invalid reports and unmet coverage constraints fail the command. Available current reports remain in the failed bundle; correct the native prerequisite or project and rerun. The shared collector owns report validation, hook ordering, bundle history and inspection. See [direct test reports](direct-tests.md) for existing-output preservation and host provenance. Host tasks can access the host network and ambient environment; these reports do not prove hermetic execution.

## Verification

`tooling/test-ant-direct.py` exercises the authored conventional/custom Ant examples and a native JUnit fixture through the compiled CLI: task discovery, real tests, current JUnit/JaCoCo, unchanged source/configuration, hooks, failed assertions and bundle inspection. `tooling/test-ant-reporting.py` separately checks the underlying native adapter. Current measured outcomes are recorded in [implementation status](../implementation-status.md); captured JAR acceptance and direct test acceptance are distinct.

Both probes passed locally on Windows with JDK 17.0.20.1, Ant 1.10.18 and JaCoCo 0.8.13, using the pinned CI prerequisite bundle. The Linux direct CLI step also passed at 173e8ca in job 110987640833; new Windows/macOS CI results remain pending. The existing captured Java suite passed at 039f83a, before this shared-launcher change; the changed captured launcher still needs its CI regression run.

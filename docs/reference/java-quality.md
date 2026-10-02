# Java quality tasks

Maven, Gradle and Ant targets expose `lint`, `format-check` and `format` without project configuration. These experimental defaults use native Checkstyle 10.21.4 and google-java-format 1.24.0, tested with JDK 17. The shared Java adapter owns file selection and native invocation; the existing task engine owns overrides, hooks, ordering and failure propagation.

```text
oyzu run list
oyzu run lint
oyzu run format-check
oyzu run format
oyzu build
```

Use target-qualified names such as `api:lint` in a multi-target workspace. Static discovery requires no Java tools and records the selected fallback linter/formatter. Execution requires provisioned tools. `lint` and `format-check` are implicit build stages; `format` is explicitly mutating and is never automatically added to the build. Failed quality gates block final artifact collection. Available native test and coverage evidence remains in `dist/manifest.json`; quality diagnostics live in action logs rather than fabricated JUnit or coverage reports.

## Provisioning and execution

Tool installation remains outside the implemented CLI scope. For development, provision a JDK 17 and the quality tools explicitly:

```text
python tooling/provision-java-quality.py --destination /absolute/path/java-quality
```

Set `OYZU_JAVA_QUALITY_HOME` to that absolute directory and put the JDK's `java` executable on `PATH`. In PowerShell, use `$env:OYZU_JAVA_QUALITY_HOME = 'E:\tools\java-quality'`; in POSIX shells, use `export OYZU_JAVA_QUALITY_HOME=/absolute/path/java-quality`. The provisioner downloads checksum-pinned upstream JARs and copies the owned Java launcher and lint defaults. It retains complete upstream archives with their embedded licenses/notices and records source URLs. Python is needed for provisioning and verification, but development quality tasks themselves require only the JDK and provisioned files. Missing or relative tool homes, missing tools, invalid Java, tool failures and extra forwarded arguments fail explicitly. Correct the prerequisite or source and rerun; the checks never repair source automatically.

The Maven, Gradle and Ant Dockerfiles provision the same tools at `/opt/oyzu-java-quality`. Build all three images from the repository root, including Ant and Gradle whose build context previously used `tooling/images`:

```text
docker build -f tooling/images/maven.Dockerfile -t oyzu-toolchain/maven:3.9.11-jdk17 .
docker build -f tooling/images/gradle.Dockerfile -t oyzu-toolchain/gradle:8.14.3-jdk17 .
docker build -f tooling/images/ant.Dockerfile -t oyzu-toolchain/ant:1.10.18-jdk17 .
```

Rebuild older images with these tags. Custom images must supply the matching JDK, JARs and `checks.xml` at the fixed tool home. Captured builds use the adapter embedded in the CLI, and the planner protects this tool-home setting from task environment replacement. Development tasks use the adapter copied during explicit provisioning; reprovision after adapter updates. Build execution does not download these tools or consult user-home caches. Image provisioning is an explicit online preparation step; quality actions run inside the existing network-disabled build sandbox. Host `oyzu run` tasks are ordinary development execution, not a claim of isolation.

## Defaults and source selection

The lint defaults check unused imports, redundant imports, empty statements and equals/hashCode consistency. Checkstyle owns parsing and rule evaluation. google-java-format owns formatting and import normalization. `format-check` invokes its native dry-run and nonzero-on-change flags; `format` invokes native replacement. See the upstream [formatter CLI](https://github.com/google/google-java-format/tree/v1.24.0) and [Checkstyle CLI](https://checkstyle.org/cmdline.html) for their native behavior.

Each task selects `.java` files recursively beneath its target, including custom source roots, reactor modules and included builds within that tree. It skips directories named `.git`, `.oyzu`, `.oyzu-build`, `.oyzu-maven`, `.gradle`, `.idea`, `target`, `build`, `dist`, `out`, `node_modules` and `.venv`. These exclusions apply at every depth. The adapter does not guess arbitrary native output directories or distinguish generated files outside these exclusions. Projects that put authored source in an excluded directory, require narrower scopes or own a different formatting style should define explicit native task replacements.

Inventory is sorted, bounded to 200,000 visited entries and 16 MiB per Java file, and does not follow symbolic links. Encountering a link outside excluded names fails instead of silently reading outside the target. Native invocations are batched within a 16,000-character argument allowance; any failed batch fails the operation. No candidate files produces an explicit not-applicable diagnostic, not invented source evidence. Build checks operate on the private workspace and do not modify the checkout. Explicit development formatting changes selected files in place and is not transactional: a later malformed file can leave earlier files formatted.

## Native configuration and overrides

Native Ant `lint`, `format-check` and `format` targets take precedence over corresponding defaults. Existing TOML task replacements and pre/post hooks work through the shared task system. For example, a project that already owns Gradle quality tasks can replace the defaults:

```toml
[tasks.lint]
argv = ["gradle", "--no-daemon", "checkstyleMain", "checkstyleTest"]

[tasks.format-check]
argv = ["gradle", "--no-daemon", "spotlessCheck"]

[tasks.format]
argv = ["gradle", "--no-daemon", "spotlessApply"]
```

Those task names and plugins must actually exist in the project; these examples do not provision them. Use `./gradlew` or `gradlew.bat` if the project uses its native wrapper. Replacement tasks must remain compatible with the captured offline toolchain when used in builds. The default launcher rejects extra arguments, including attempts to pass `--replace` to `format-check`; put native flags in an explicit replacement instead.

This increment does not yet infer Maven quality-plugin bindings, Gradle quality tasks, arbitrary Checkstyle XML/suppression files, Kotlin/Groovy quality tools or per-module formatter differences. Configured checks still run through their native build lifecycle, and can coexist with the defaults; use replacements when their rules differ. It is not a claim of full Java scenario acceptance or all Java language versions.

## Verification scope

`python tooling/test-java-quality.py --cli <compiled-oyzu> --tools <quality-home>` exercises real native tools through the CLI without Maven, Gradle or Ant execution. Windows/JDK 17 verification passed task discovery without tools, native lint and formatting failures, unchanged source during checks, explicit formatting, custom source directories, output exclusions, missing prerequisites and task replacements. CI runs this probe on Windows, macOS and Linux after compiling the CLI.

The Linux captured suite additionally requires successful snapshot artifacts and retained test/coverage evidence, then introduces compilable lint and formatting violations and requires failed quality actions, blocked artifacts and unchanged source. These new isolated cases await CI confirmation; host checks do not establish captured-build acceptance. Track results in [implementation status](../implementation-status.md).

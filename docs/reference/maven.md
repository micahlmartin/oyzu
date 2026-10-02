# Maven builds

Status: experimental. Oyzu detects a `pom.xml` without a build YAML file and treats a native Maven reactor as one target. Maven owns inheritance, active profiles, module order, dependency resolution and lifecycle execution; Oyzu captures inputs, projects snapshot versions, enforces isolated execution and collects evidence. The [reactor example](../../examples/builds/java-maven-reactor/README.md) includes unit/integration reporting variations.

## Tools and commands

Development tasks use provisioned Maven or the native Maven wrapper. Static `oyzu run list` does not execute Maven. Implicit `install` runs dependency preparation, `build` runs `mvn -B verify`, and `test` runs `mvn -B test`. The explicit test task is not separately repeated during the build lifecycle. Checks already bound to `verify` run under Maven; default Java lint/format tooling is not implemented yet.

Captured builds require Docker and the explicitly provisioned Maven 3.9.11/JDK 17 image:

```text
docker build -f tooling/images/maven.Dockerfile -t oyzu-toolchain/maven:3.9.11-jdk17 .
oyzu -C examples/builds/java-maven-reactor/project run list
oyzu -C examples/builds/java-maven-reactor/project build
```

Rebuild this image for the native test-plan extension; an older image with the same tag cannot produce the required metadata. Oyzu does not pull a missing image or install Maven. Custom `--image maven=<image>` toolchains need the matching extension, native tools and Python runtime. Windows native metadata/lifecycle probes and Linux captured builds are separate verification scopes; a portable CLI does not establish a native isolated executor on every host.

## Preparation and outputs

Preparation captures native POMs and evaluates the reactor inside the constrained toolchain. Native version projection preserves each module's base version with a source-derived suffix such as `0.1.0-dev.g<source-prefix>` and updates local reactor references in private POMs. The checkout stays unchanged. Supported artifact packaging is `pom`, `jar` and `war`; other packaging types fail explicitly. Outputs retain native final names and carry planned snapshot versions in the bundle.

The standalone source profile routes Maven through a loopback mirror backed by Oyzu's scoped Maven Central broker. Upstream requests occur during acquisition; the captured repository is then copied into a private workspace for offline execution. Custom/private repositories, corporate connector credentials and complete dependency-edge evidence are not implemented by this profile. Maven has no general lockfile here: the prepared repository identity records the actual captured inputs and is not a claim that independent future acquisition resolves identically.

One offline lifecycle runs JaCoCo 0.8.13 agent preparation, native `verify` and JaCoCo reporting. A failed lifecycle attempts coverage reporting without rerunning tests and preserves failure. Required reports, logs, artifacts and their digests are recorded under `dist/` and in `dist/manifest.json`. Any failed gate or invalid/missing required report prevents final artifact collection. `oyzu inspect dist` verifies bundle integrity, not release eligibility. Publishing, signing and production promotion are outside this builder increment.

## Native test reports

After acquisition, the extension asks Maven for the effective `verify` execution plan. It records report directories for bound Surefire `test` and Failsafe `integration-test` goals using Maven's plugin configuration and expression evaluator. Defaults, inherited configuration, execution-specific settings and paths with spaces remain native Maven behavior. The directory plan is frozen in prepared metadata; shared planning does not parse POM expressions or guess test-class filename patterns.

Before execution, the adapter removes old `TEST-*.xml` outputs from those declared directories in the private workspace. After the lifecycle it copies produced XML unchanged into `.oyzu-maven/reports/<reactor-index>/`, using source-path hashes to avoid filename collisions. The shared report collector validates every retained XML file and records its actual results. Duplicate native directories are collected once. Individual Surefire/Failsafe directories may be empty: a module with only integration tests does not need fabricated unit-test XML. Modules with captured test sources require at least one native JUnit file across the combined group, plus their JaCoCo report. If every engine leaves no XML, disabled reporting or skipped tests fail that obligation; malformed XML also fails. The combined group records available native files and does not prove per-engine completeness when a custom reporter suppresses some outputs. No passing report is synthesized.

Report paths must remain inside the captured workspace, outside reserved `.oyzu-maven` state, without redirected directories. Different modules must use distinct native report directories; shared paths fail planning rather than attributing the same tests to multiple modules. Each XML file is bounded to 16 MiB and each module's group to 64 MiB. Native test failures retain XML and available coverage while blocking artifacts. Fix the test, native report configuration or missing evidence and rebuild; prior successful bundles remain history rather than satisfying the current invocation. Native goals within one module may rewrite a shared report filename; Oyzu retains the final native files, not invented per-execution copies.

Prepared layout version 2 requires the frozen directory plan; regenerate older prepared inputs and rebuild the toolchain image. Retained test-report paths change because collection now combines native engines. Consumers should follow manifest report records rather than assume Surefire-specific paths. JaCoCo remains at the module's conventional `target/site/jacoco/jacoco.xml` beneath its effective build directory; custom coverage destinations, aggregate reactor coverage and test source roots added only during later lifecycle execution need further integration.

## Verification and limits

The native extension can be exercised with provisioned Maven/JDK tools:

```text
python tooling/test-maven-metadata.py --maven-home <maven-home> --java-home <jdk-home>
python tooling/test-maven-metadata.py --maven-home <maven-home> --java-home <jdk-home> --test-lifecycle --repository <prepared-native-repository>
```

The default check needs no downloaded plugins. The lifecycle variant needs its pinned fixture plugins/dependencies in that native repository; adding `--acquire` explicitly allows a native online fixture run to provision them before the offline checks. That test setup is distinct from Oyzu's captured acquisition protocol.

Windows native testing passed effective custom report directories, successful/failed unit and integration reports, unchanged XML bytes and an integration-only module. The Linux captured suite also exercises snapshot artifacts, source immutability, module coverage and failed integration-test evidence; the new report-plan cases await CI confirmation. Earlier reactor artifact/report evidence remains recorded in [implementation status](../implementation-status.md). Remaining work includes default Java quality tasks, broader test providers, custom/aggregate coverage, generated test roots, private repositories, release policy and complete authored-scenario acceptance.

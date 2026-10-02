# Helm charts and native chart tests

The experimental `helm/chart` builder discovers an application or library chart at `Chart.yaml` or `chart/Chart.yaml`. A basic chart needs no Oyzu configuration. If both locations exist, choose explicit target paths in `build.yaml`; ambiguous discovery fails. The [chart example](../../examples/builds/helm-chart/README.md) includes a contained local library dependency and optional native assertion suites.

## Prerequisites and commands

Development commands use an already provisioned Helm executable. Native assertion suites additionally require the `helm-unittest` plugin in Helm's configured plugin directory. Oyzu does not install either tool. Local verification used Helm 3.22.0 and helm-unittest 1.2.0 on Windows; Linux native checks are wired into CI. This is not a claim of verified plugin execution on all three hosts.

```text
oyzu run list
oyzu run test
oyzu run lint
oyzu build --plan
oyzu build
oyzu inspect dist
```

`oyzu run list` discovers tasks without launching Helm or interpreting assertion YAML. The initial group is `project`; explicit targets use their own names. Native development tasks are `install` (`helm dependency build`), `build` (`helm package`), `lint` (`helm lint`) and `test`. These development commands have native effects: install may acquire dependencies and update locks, and package writes a chart archive. Direct test execution currently retains native console output/exit status rather than collecting a `dist/` report bundle.

Captured builds require Docker and an explicitly provisioned toolchain image:

```text
docker build -f tooling/images/helm.Dockerfile -t oyzu-toolchain/helm:3.22.0 .
```

This repository provisioning step downloads checksum-pinned Helm and unittest distributions. The CLI does not download toolchain images or plugins during a build. The current image is Linux amd64 and includes Python 3.12 for the owned reporting/archive adapters. Its plugin directory is `/opt/oyzu-helm-plugins`; the build plan fixes `HELM_PLUGINS` to that toolchain-owned location so a task environment override cannot redirect it to project code. Custom images must provide the compatible executables and directory layout. Image identity is captured in build evidence.

## Inference and native assertions

Without native suites, the implicit test validates local rendering and values schemas for an application chart. A library chart uses strict native lint with subcharts, because it cannot be rendered as an installable application. Captured application rendering uses `helm template --dry-run=client`, an empty kubeconfig and no cluster access. This is chart validation, not a claim that a deployed application works.

An immediate `tests/*_test.yaml` file under the selected chart or an unpacked subchart in `charts/<name>/` activates helm-unittest automatically. Nested unpacked subcharts are included even when the parent has no suite. Each traversed subchart must have a regular `Chart.yaml`; dot/underscore-prefixed chart directories are ignored. Discovery records the selected framework and suite evidence; Helm unittest owns YAML parsing, assertions, test selection and native results. Malformed YAML remains a native test failure, rather than being interpreted by a new Oyzu test language. Detection permits up to 128 suite files and 4096 inspected test/subchart-directory entries across the selected chart tree, at most 16 subchart levels, with bounded source reads. Nonportable paths, symlinked suites/directories and invalid source types fail discovery.

For example, `chart/tests/deployment_test.yaml` can contain:

```yaml
suite: deployment contract
templates:
  - templates/deployment.yaml
tests:
  - it: uses one replica
    asserts:
      - equal:
          path: spec.replicas
          value: 1
```

No test section is added to `build.yaml`. `oyzu run test` now invokes `helm unittest --strict <chart>`. Captured builds run both the independent render/schema validation and native unittest, requesting the plugin's JUnit output. Native subchart behavior remains enabled when the plugin is selected. Test suites remain source inputs; use native `.helmignore` with `tests/` if they should be excluded from the published chart archive. Oyzu does not silently rewrite the chart's packaging rules.

## Build outputs and failure handling

Preparation captures the chart's contained local `file://` dependency closure, rejects cycles/escapes and runs native dependency preparation offline. Existing locks are honored; stale locks fail. Generated lock timestamps and chart archive metadata are normalized. The root chart version gains the source-derived `-dev.g...` snapshot suffix; native `appVersion` and dependency versions retain their own meanings.

The build packages the chart, runs testing and strict lint, then collects successful outputs. Applications produce a snapshot `.tgz` and `rendered.yaml`; libraries produce the snapshot `.tgz` only. A failing check prevents final artifact collection. Build-time rendering, generated files and snapshot projection take place in private captured workspaces, leaving the checkout unchanged. Helm formatting is not currently implemented; there is no fabricated successful format task.

Shared bundle collection retains reports beneath `dist/`, and `dist/manifest.json` records their paths, content digests and parsed summaries:

- `junit.xml` records one native render/schema or library-validation check.
- `unittest.xml` is required when native suites are selected and retains native test cases, failures and skips. It is independent of the render result.
- The target's coverage applicability is explicitly `inapplicable`: chart assertions have no application-source coverage denominator. No synthetic coverage percentage or application coverage file is produced.

The manifest also records framework discovery, action outcomes and successful artifact identities. Render success cannot override assertion failure. Missing/invalid required reports, plugin launch failure, native failure and timeout fail testing. Stale report files are removed before invocation; prior success cannot satisfy a failed launch. Native validation and unittest each have a 120-second adapter timeout, in addition to executor lifecycle controls. Inspect retained task diagnostics and reports, correct the native suite/toolchain/values or lock input, and rebuild.

Shared TOML task replacement and hooks still apply. Replacing a test command does not remove its required report obligations. A replacement that returns zero without producing the required reports fails collection. Inspecting bundle integrity does not certify release eligibility or production trust.

## Snapshot expectations are inputs

Helm unittest can create missing `matchSnapshot` baselines even without `--update-snapshot`. Captured testing compares the path/content identities of files under native `__snapshot__` directories before and after unittest. Creating, changing or deleting a baseline fails the task and prevents artifact collection. The original native JUnit is retained: it may show a passing assertion while the action fails the baseline-integrity check. Read both action status and reports.

Generate or update baselines deliberately using native Helm unittest outside the captured build, review the resulting files and commit them with the suite. Then a matching baseline passes, while a changed rendered value fails normally. Direct `oyzu run test` preserves native behavior, including possible snapshot creation; it is a development operation, not captured build evidence.

The baseline inventory rejects symbolic links, unreadable traversal and nonregular baseline files, and bounds scanning to 100000 chart entries, 16 MiB per baseline file and 64 MiB total baseline contents. Exceeding these limits fails the task. This checks persistent baseline changes; it is not a general filesystem write monitor.

## Verification and remaining work

```text
python tooling/test-helm-reporting.py --helm <provisioned-helm> --cli <compiled-oyzu>
python tooling/test-helm-archive.py
```

The native reporting probe exercises static CLI listing with an empty PATH, successful and failed native assertions (including subchart-only suites), malformed suites, independent render/schema results, library validation, missing-tool stale-output rejection, missing snapshot creation, matching reviewed baselines and native snapshot mismatches. Rust tests cover discovery, dependency containment, planning, required report declarations and fixed plugin location. CI captured-build cases additionally require snapshot chart/rendered artifacts, manifest evidence, unchanged source and artifact rejection after assertion or baseline-integrity failure; root-suite/baseline cases passed run 36967080170 at `51464a9`; the newer subchart-only assertions still require CI confirmation. See [implementation status](../implementation-status.md) for revision-specific results.

Automatic discovery of custom suite globs, templated test charts, suites inside packaged subchart archives and suites in unprepared sibling `file://` dependencies remains unfinished. Static discovery follows unpacked chart directories already present under the selected chart; it does not unpack archives or acquire dependencies. Other plugins, remote/OCI chart dependency acquisition, image-artifact bindings, Helm formatting, publishing/signing and broader platform execution remain required work. These limitations do not change the full builder/scenario objective.

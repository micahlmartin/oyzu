# Helm charts and native chart tests

The experimental `helm/chart` builder discovers an application or library chart at `Chart.yaml` or `chart/Chart.yaml`. A basic chart needs no Oyzu configuration. If both locations exist, choose explicit target paths in `build.yaml`; ambiguous discovery fails. The [chart example](../../examples/builds/helm-chart/README.md) includes a contained local library dependency and optional native assertion suites.

Vendored children under `charts/` must also be declared in the parent's native `Chart.yaml` dependencies for strict lint. For a contained child, declare its name/version and omit the repository URL. Passing child assertions does not waive this native metadata requirement. If lint reports missing dependencies, correct the parent declaration; Oyzu does not silently add native dependencies or disable the lint gate.

## Prerequisites and commands

Development commands use an already provisioned Helm executable. Direct test/report execution requires Python 3.11+ as `python` on PATH; native assertion suites also require the `helm-unittest` plugin in Helm's configured plugin directory. Oyzu does not install either tool. Local verification used Helm 3.22.0 and helm-unittest 1.2.0 on Windows; Linux native checks are wired into CI. This is not a claim of verified plugin execution on all three hosts.

```text
oyzu run list
oyzu run test
oyzu run lint
oyzu run format-check
oyzu run format
oyzu build --plan
oyzu build
oyzu inspect dist
```

`oyzu run list` discovers tasks without launching Helm or interpreting assertion YAML. The initial group is `project`; explicit targets use their own names. Native development tasks are `install` (`helm dependency build`), `build` (`helm package`), `lint` (`helm lint`) and `test`. These development commands have native effects: install may acquire dependencies and update locks, and package writes a chart archive. Direct `test` now writes a test-only `dist/` bundle with native validation JUnit, an additional independent unittest JUnit report when suites are detected, logs and a manifest. It records coverage as inapplicable, produces no chart artifact and uses the shared hook/report/bundle workflow. Both default validation and unittest run against private chart copies that exclude `.oyzu`, `.git` and existing `dist` output; root charts therefore do not expose the live bundle lease to Helm. Native dependencies must already be prepared (for example with `oyzu run install`); testing does not acquire them. Run `oyzu inspect dist` to inspect the retained evidence. The default adapter accepts no extra native flags; use an explicit task replacement and report declarations for customized arguments. See [direct test reports](direct-tests.md).

Captured builds require Docker and an explicitly provisioned toolchain image:

```text
docker build -f tooling/images/helm.Dockerfile -t oyzu-toolchain/helm:3.22.0 .
```

This repository provisioning step downloads checksum-pinned Helm, unittest and yamlfmt distributions. The CLI does not download toolchain images or plugins during a build. The current image is Linux amd64 and includes Python 3.12 for the owned reporting/archive/formatting adapters and yamlfmt 0.21.0. Its plugin directory is `/opt/oyzu-helm-plugins`; the build plan fixes `HELM_PLUGINS` to that toolchain-owned location so a task environment override cannot redirect it to project code. Custom images must provide the compatible executables and directory layout. Image identity is captured in build evidence.

## YAML formatting

Discovery supplies a read-only `format-check` build stage and an explicit, source-mutating `format` development task. Both use native yamlfmt through an owned Python adapter. Development execution requires Python 3.11+ as `python` and yamlfmt on PATH; verification uses yamlfmt 0.21.0. Missing tools fail with process diagnostics. The repository's `python tooling/provision-yamlfmt.py --destination <directory>` is an explicit maintainer/CI provisioning step that verifies pinned release digests and retains the upstream license. It is not invoked by the CLI. Put that directory on PATH before running development formatting.

The adapter selects `.yaml`/`.yml` files immediately inside chart roots identified by `Chart.yaml`, plus YAML below their `tests/` and `crds/` directories. It includes unpacked subcharts and contained sibling charts within the target root. Go templates under `templates/`, native `__snapshot__` baselines, packaged chart archives, `Chart.lock`, generated output and hidden directories are excluded. Formatting does not edit or repack dependency archives. Template formatting remains unfinished; a passing YAML check makes no claim about Go-template style.

An explicit private yamlfmt configuration fixes LF line endings and a final newline while retaining native basic-formatter defaults. Parent/home formatter configuration is ignored. Target-local `.yamlfmt`, `.yamlfmt.yaml`, `.yamlfmt.yml`, `yamlfmt.yaml` or `yamlfmt.yml` currently fails the implicit operation with guidance to override the affected tasks; native custom formatter configuration support remains unfinished. Use existing TOML task overrides when customization is necessary, without adding a build-file formatting DSL.

`format-check` lists differing paths and exits unsuccessfully without changing files. Invalid YAML also fails with the native diagnostic. Run `oyzu run format`, review the changes and retry the check/build. Explicit formatting writes changed files as it proceeds; if a later file fails, earlier formatting edits remain. Neither command downloads dependencies or contacts a cluster. Selection rejects links/reparse points and nonregular YAML inputs, with limits of 100000 traversed entries, 4 MiB per selected file and 64 MiB total. Native formatter invocations each have a 30-second timeout. Host tasks are not a security sandbox; captured builds use the executor's isolation.

## Inference and native assertions

Without native suites, the implicit test validates local rendering and values schemas for an application chart. A library chart uses strict native lint with subcharts, because it cannot be rendered as an installable application. Captured application rendering uses `helm template --dry-run=client`, an empty kubeconfig and no cluster access. This is chart validation, not a claim that a deployed application works.

An immediate `tests/*_test.yaml` file under the selected chart or an unpacked subchart in `charts/<name>/` activates helm-unittest automatically. Nested unpacked subcharts and suites inside local `charts/*.tgz` archives are included even when the parent has no suite. Archive evidence binds the original archive digest and a `tar:` member location (nested archive boundaries use `!/`). Each traversed subchart must have a regular `Chart.yaml`; dot/underscore-prefixed chart directories are ignored. Discovery records the selected framework and suite evidence; Helm unittest owns YAML parsing, assertions, test selection and native results. Malformed YAML remains a native test failure, rather than being interpreted by a new Oyzu test language. Detection permits up to 128 suite files and 4096 inspected test/subchart-directory entries across the selected chart tree, at most 16 subchart levels, with bounded source reads. Nonportable paths, symlinked suites/directories and invalid source types fail discovery.

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

Archive inspection is static and never extracts into the checkout. Archive inputs share the 4 MiB metadata-file bound; expanded archive bytes across one discovery scope are bounded to 64 MiB and 4096 entries, with 16 levels of archive nesting and 128 total suites. One chart root with `Chart.yaml` is required. Nonregular entries, links, duplicate/case-alias paths, nonportable paths and mixed roots fail. These constraints admit native Helm-generated archives, not arbitrary tar layouts.

No test section is added to `build.yaml`. `oyzu run test` displays the logical `helm unittest --strict <chart>` task; its adapter expands packaged subcharts in a private copy before invoking the native plugin. Captured builds run both the independent render/schema validation and native unittest, requesting the plugin's JUnit output. Native subchart behavior remains enabled when the plugin is selected. Test suites remain source inputs; use native `.helmignore` with `tests/` if they should be excluded from the published chart archive. Oyzu does not silently rewrite the chart's packaging rules.

Helm can interpret non-chart files beneath `charts/` as dependencies. Keep checksum metadata outside that directory or exclude sidecars using native `.helmignore`, such as `charts/*.tgz.sha256`. The packaged-only example retains its fixture checksum in source and excludes it from native chart loading/packaging. Its host probe now exercises the exact checked-in chart through lint, rendering, assertions and private dependency preparation, including the sidecar.

## Build outputs and failure handling

Preparation expands contained prepackaged subcharts in its private workspace and captures the chart's contained local `file://` dependency closure, rejects cycles/escapes and runs native dependency preparation offline. Existing locks are honored; stale locks fail. Generated lock timestamps and chart archive metadata are normalized. Expansion never overwrites an existing chart directory; ambiguous packed/unpacked copies fail. Prepared layout version 2 records this behavior; regenerate older prepared dependencies. Each source archive remains unchanged in the checkout. The root chart version gains the source-derived `-dev.g...` snapshot suffix; native `appVersion` and dependency versions retain their own meanings.

The build packages the chart, runs testing, strict lint and read-only YAML formatting, then collects successful outputs. Applications produce a snapshot `.tgz` and `rendered.yaml`; libraries produce the snapshot `.tgz` only. A failing check prevents final artifact collection. Build-time rendering, generated files and snapshot projection take place in private captured workspaces, leaving the checkout unchanged. Formatting checks the captured source YAML, not the regenerated dependency chart or its projected version. This adds a default gate: previously accepted charts with formatting differences now need explicit formatting or a task override.

Shared bundle collection retains reports beneath `dist/`, and `dist/manifest.json` records their paths, content digests and parsed summaries:

- `junit.xml` records one native render/schema or library-validation check.
- `unittest.xml` is required when native suites are selected and retains native test cases, failures and skips. It is independent of the render result.
- The target's coverage applicability is explicitly `inapplicable`: chart assertions have no application-source coverage denominator. No synthetic coverage percentage or application coverage file is produced.

The manifest also records framework discovery, action outcomes and successful artifact identities. Render success cannot override assertion failure. Missing/invalid required reports, plugin launch failure, native failure and timeout fail testing. Selected unittest execution must produce at least one native test case; a zero-test success is rejected. Stale report files are removed before invocation; prior success cannot satisfy a failed launch. Native validation and unittest each have a 120-second adapter timeout, in addition to executor lifecycle controls. Inspect retained task diagnostics and reports, correct the native suite/toolchain/values or lock input, and rebuild.

Shared TOML task replacement and hooks still apply. Replacing a test command does not remove its required report obligations. A replacement that returns zero without producing the required reports fails collection. Inspecting bundle integrity does not certify release eligibility or production trust.

## Snapshot expectations are inputs

Helm unittest can create missing `matchSnapshot` baselines even without `--update-snapshot`. Oyzu unittest execution compares the path/content identities of files under native `__snapshot__` directories before and after unittest. Creating, changing or deleting a baseline fails the task and prevents artifact collection. The original native JUnit is retained: it may show a passing assertion while the action fails the baseline-integrity check. Read both action status and reports.

Generate or update baselines deliberately using native Helm unittest outside the captured build, review the resulting files and commit them with the suite. Then a matching baseline passes, while a changed rendered value fails normally. Direct `oyzu run test` now uses the same private-copy baseline check and also rejects creation or alteration. Update baselines deliberately with the native Helm command, not this read-only Oyzu operation. This is a compatibility change from earlier direct task execution; development results still are not captured build evidence.

The private test copy is bounded to 100000 entries and 64 MiB of files, excludes `.git`, `.oyzu` and `dist` directories, and rejects links/reparse points. The copy includes the chart tree only; relative references to external files are not transported. Development execution remains a host process, not a filesystem or network sandbox. Captured builds retain the executor's isolation. Archive expansion uses the same 4 MiB compressed/64 MiB expanded bounds and refuses collisions.

The baseline inventory rejects symbolic links, unreadable traversal and nonregular baseline files, and bounds scanning to 100000 chart entries, 16 MiB per baseline file and 64 MiB total baseline contents. Exceeding these limits fails the task. This checks persistent baseline changes; it is not a general filesystem write monitor.

## Verification and remaining work

```text
python tooling/test-helm-reporting.py --helm <provisioned-helm> --cli <compiled-oyzu>
python tooling/test-helm-archive.py
python tooling/test-helm-quality.py --cli <compiled-oyzu> --yamlfmt <provisioned-yamlfmt>
```

The native reporting probe exercises static CLI listing with an empty PATH, successful and failed native assertions (including subchart-only suites), malformed suites, independent render/schema results, library validation, missing-tool stale-output rejection, missing snapshot creation, matching reviewed baselines and native snapshot mismatches. Packaged-only native probes passed real assertions and failures, unchanged source archives, declared local archive dependencies, empty-suite rejection and read-only baseline handling on Windows. Static archive tests cover nested evidence and rejected unsafe/oversized inputs; these new captured scenarios await Linux CI. Rust tests cover discovery, dependency containment, planning, required report declarations and fixed plugin location. CI captured-build cases additionally require snapshot chart/rendered artifacts, manifest evidence, unchanged source and artifact rejection after assertion or baseline-integrity failure; root-suite/baseline cases passed run 36967080170 at `51464a9`; the newer subchart-only assertions still require CI confirmation. See [implementation status](../implementation-status.md) for revision-specific results.

The compiled-CLI quality probe checks implicit task metadata, native read-only checks, explicit formatting, unchanged templates/baselines, invalid YAML, ambient configuration isolation, custom-config rejection and task overrides against authored charts. It is wired into the three-host task jobs. The captured Helm suite additionally requires a successful format action and proves unformatted values block artifact collection without changing source; this new gate awaits Linux CI evidence. The earlier chart/assertion scenarios, including packaged subcharts, passed Linux job 110767008279 in [run 36983990838](https://github.com/micahlmartin/oyzu/actions/runs/36983990838), resolving the earlier pending subchart acceptance above.

Automatic discovery of custom suite globs, templated test charts and suites in unprepared sibling `file://` dependencies remains unfinished. Static discovery follows unpacked chart directories and inspects local chart archives already present under the selected chart; it does not extract archives or acquire dependencies. Other plugins, remote/OCI chart dependency acquisition, image-artifact bindings, Go-template formatting, native formatter customization, publishing/signing and broader platform execution remain required work. These limitations do not change the full builder/scenario objective.

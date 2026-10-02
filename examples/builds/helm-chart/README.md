# EX-028: Helm packaging with a local library dependency

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- Prepare the chart's local dependency and honor its generated lock.
- Lint/render/package charts; never deploy to a cluster.

## Review the project

Open the checked-in project roots: `project`. Source, native manifests, and Oyzu configuration are included here so we can review the intended experience directly.

The intended Oyzu interface is:

```text
oyzu build
```


No build YAML is required for this scenario. Configuration, where present, demonstrates only the feature under discussion.

## Native checks available now

With the named toolchains already installed, run:

```text
# From project
helm dependency build chart

# From project
helm lint chart

# From project
helm template example chart
```

Native commands document the underlying ecosystem workflow. They are supporting context, not a prerequisite for accepting or revising this design contract.

## Failure and variation cases

- **invalid-values:** Set replicaCount to 0. Expected: Schema validation fails before packaging success is reported.
- **unformatted-values:** Change `replicaCount: 1` to `replicaCount:    1`. The read-only `format-check` task must fail and block artifact collection without changing source. With yamlfmt provisioned, `oyzu run format` deliberately repairs YAML formatting. Templates and snapshot baselines remain untouched; see the [formatting scope](../../../docs/reference/helm.md#yaml-formatting).
- **native-unit-suite:** Copy [deployment_test.yaml](variants/deployment_test.yaml) to `project/chart/tests/deployment_test.yaml`, and add `tests/` to the chart's native `.helmignore` if tests should be excluded from its archive. With helm-unittest provisioned, `oyzu run list` selects `helm unittest` and a build retains both native suite JUnit and separate render/schema JUnit. Changing the expected replica value to 99 must fail testing and prevent artifact collection. This uses native Helm test YAML, not an Oyzu assertion language. See the [Helm reference](../../../docs/reference/helm.md) for current limits.
- **missing-snapshot-baseline:** Copy [deployment_snapshot_test.yaml](variants/deployment_snapshot_test.yaml) into `project/chart/tests/` without a baseline. A captured build must fail when native unittest generates its missing expectation, even if native JUnit reports success. Generate baselines explicitly with native Helm unittest, review them and check them in before building. A reviewed matching baseline passes; a changed deployment specification fails.

- **subchart-only:** Run from [variants/subchart-only](variants/subchart-only/Chart.yaml). The parent has no suite; the unpacked child supplies the native assertion. Discovery must select unittest, retain render and assertion JUnit, and produce snapshot chart/rendered artifacts. Change the child expectation from `"42"` to `"99"`: testing must fail and artifacts must not be collected.

## Contract and limitations

Acceptance criteria: BUILDER-01, BUILDER-05. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

## Packaged subchart assertions

Both child-only variants declare the vendored child in the parent's native `Chart.yaml`. No repository URL is needed. Removing that declaration must fail native strict lint even if all child assertions pass; the CLI build keeps lint as a required gate.

`variants/packaged-only` contains a native Helm archive built from `variants/subchart-only/charts/child`. Its parent declares that local dependency without a repository. Run Oyzu directly from the packaged-only variant to exercise static archive-suite discovery, private native dependency preparation, real assertions and snapshot chart/rendered artifacts. No extraction occurs in the checkout. Recreate the child archive using `helm package`, then the owned archive normalization helper if maintaining the fixture; refresh its generated `.tgz.sha256` identity sidecar and do not hand-author archive bytes. Structural example validation checks that identity; native archive probes verify its behavior. A changed child assertion must fail testing and block artifact export. Empty selected suites also fail, even if the native plugin reports a successful exit with zero tests.

The variant's native `.helmignore` excludes checksum sidecars from chart loading and packaging. Keep the sidecar in source for fixture verification; Helm otherwise interprets files under `charts/` as subcharts. Native probes exercise this exact checked-in variant as well as generated chart cases.

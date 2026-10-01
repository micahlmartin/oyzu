# EX-029: Build a Go artifact, materialize it, then package a chart

The [build file](project/build.yaml) selects linux/amd64 on the image consumer and materializes the app's primary artifact at bin/server. No producer platform declaration, duplicate depends_on, manual copy task, or producer dist path is required.

```yaml
image:
  uses: docker/image
  platform: linux/amd64
  materialize:
    - from: app
      to: bin/server
```

The ordinary [Dockerfile](project/Dockerfile) uses COPY bin/server /server. Oyzu prepares that isolated path after building/testing or retrieving an eligible matching artifact. It preserves the exact artifact digest and executable metadata.

The [expected context](expected-materialization.json) records platform and path relationships. A Windows or wrong-architecture artifact cannot satisfy this consumer. These Go sources need no external runtime library; the builder must still check that scratch is suitable.

The chart still depends on the image. Passing its digest into generated chart values is a separate, still-proposed value-binding mechanism in [artifact-bindings.json](artifact-bindings.json). Materialize does not silently mutate chart source.

Intended command: `oyzu build`. Supporting native checks are `go test ./...` from project/app and `helm lint .` from project/chart. Native Docker alone does not provide materialization.

Failure cases include digest mismatch, wrong platform/ABI, path escapes, collisions, and unavailable required target tests. See [scenario.json](scenario.json) and the [shared contract](../../MATERIALIZATION.md).

Acceptance: BUILDER-05, BUNDLE-01, PLAN-07, PLAN-08, BUILDER-07. Implementation remains pending.

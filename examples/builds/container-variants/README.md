# EX-027: Platform-matched artifacts for multi-platform images

The agreed design uses one Go producer and an image target with a platform matrix. See the [shared contract](../../MATERIALIZATION.md) and [expected contexts](expected-materialization.json).

```yaml
api:
  uses: go/app
  path: api
image:
  uses: docker/image
  matrix:
    platform: [linux/amd64, linux/arm64]
  materialize:
    - from: api
      to: bin/server
```

Run `oyzu build`. The intended graph is:

```text
api [linux/amd64] -> image [linux/amd64]
api [linux/arm64] -> image [linux/arm64]
```

Each context contains its matching binary at bin/server. The same Dockerfile copies it to /server. These programs use only Go's standard library and need no external shared-library dependency; other runtime requirements must still be checked before choosing scratch.

The invoking workstation can be Windows, macOS, or Linux. Its host executable is never substituted for the required Linux target. The binary prints its actual compiled GOOS/GOARCH, making target identity observable on a suitable executor.

Image variants and digests remain separate and can be combined into an OCI index. Required ARM tests need a suitable executor; cross-compilation alone cannot satisfy them.

Negative cases cover incompatible cached variants, target capability gaps, ABI mismatch, collisions, and path escapes. See [scenario.json](scenario.json).

Acceptance: PLAN-05, EXEC-04, PLAN-07, PLAN-08, BUILDER-07. This is the agreed example contract, not verified Oyzu behavior.

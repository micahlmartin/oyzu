# EX-049: Select multiple artifacts from one producer

The Go project has cmd/server and cmd/migrate. Their names identify two discovered runnable artifacts; there is no unique primary executable.

```yaml
tools:
  uses: go/app
  path: tools
image:
  uses: docker/image
  platform: linux/amd64
  materialize:
    - from: tools
      artifact: server
      to: bin/server
    - from: tools
      artifact: migrate
      to: bin/migrate
```

Run `oyzu build`. The image's Linux requirement flows to both selected artifacts. The reference implies the dependency; no duplicate depends_on is needed.

The [Dockerfile](project/Dockerfile) uses stable context paths. Producer output filenames, cache locations, and versioned dist paths never appear in it. Executable metadata and original identity survive materialization.

Removing a selector must report ambiguity. An unknown selector, path escape, collision, or incompatible cached variant must fail. See [expected-materialization.json](expected-materialization.json) and [scenario.json](scenario.json).

A supporting native command is `go test ./...` from project/tools. The simple standard-library commands can be built statically for Linux; adding native dependencies requires runtime compatibility checks.

Acceptance: PLAN-07, PLAN-08, BUILDER-07. This locks in the [example contract](../../MATERIALIZATION.md), not a claim that Oyzu is implemented.

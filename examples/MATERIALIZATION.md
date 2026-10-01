# Materialization and target platforms

This is the agreed example contract from the design discussion. Product implementation remains pending; the containing OEPs are still drafts with other open decisions.

## The developer-facing shape

```yaml
api:
  uses: go/app

image:
  uses: docker/image
  platform: linux/amd64
  materialize:
    - from: api
      to: bin/server
```

`from` selects the producer target's unambiguous primary artifact. Optional `artifact` selects a named output when the target produces several. `to` is relative to the consumer's isolated input workspace. For Docker, that workspace provides the build context, so an ordinary `COPY bin/server /server` works.

The reference creates a dependency. There is no duplicate depends_on, interpolation language, copy task, or knowledge of the producer's internal output path. Materialization may occur on a local or remote executor; it does not require intermediate files on the developer's machine. The final dist bundle remains a separate export.

## Platform selection

The consumer requires a compatible artifact for its `platform`. With `matrix.platform`, each image variant gets matching inputs at the same logical paths in a separate workspace. A Windows invocation can therefore request Linux artifacts without selecting a cached Windows executable.

Host means where Oyzu was invoked; execution means where an action runs; target means where its output must work. Builders choose supported compilation/execution strategies. Build-time tools follow the execution platform rather than blindly inheriting the runtime artifact's target.

OS/architecture alone is not sufficient for native dependencies. Runtime/ABI constraints, libraries, toolchains, and base-image compatibility must agree. An unsupported cgo cross-compiler or target test executor produces a capability failure. Cross-compilation never implies tests ran on that target.

Platform-independent outputs can be shared only when the builder establishes that property and relevant inputs match. Arbitrary Node build scripts are not automatically platform independent.

## Paths and integrity

File artifacts map to exact filenames. Directory contents appear under the requested destination, so `to: site` exposes `site/index.html`, not `site/dist/index.html`. No implicit archive extraction is performed.

The planner fixes logical identity, destinations, and compatibility requirements. Content digests are verified when outputs become available. Filesystem placement preserves required executable metadata; copies or storage optimizations must not expose mutable producer state to a consumer.

Reject missing/ambiguous artifacts, path escapes, escaping links, source collisions, and overlapping destinations. Build-context ignore rules apply to ordinary source files; explicitly materialized inputs must remain present or trigger an error. Files are never injected into the source checkout, and consumers never scrape dist.

## Managed and standalone operation

Managed defaults/constraints can select permitted platforms, approved toolchains and bases, execution capabilities, and required tests. Omitted settings resolve visibly into the frozen plan. The standalone container-platform default is still an explicit open decision. Choosing Linux locally does not grant CI or production trust.

## Examples

| Contract | Example |
| --- | --- |
| One binary, fixed Linux target, Helm consumer | [EX-029](builds/image-and-chart/README.md) |
| Matching binaries for two image platforms | [EX-027](builds/container-variants/README.md) |
| Materialization in a mixed monorepo | [EX-030](builds/mixed-monorepo/README.md) |
| Select two outputs from one target | [EX-049](builds/materialize-selected-artifacts/README.md) |
| Directory placement and reusable platform-independent content | [EX-050](builds/materialize-directory/README.md) |

Chart image-digest values are not file materialization. [OEP-0018](../docs/proposals/OEP-0018-container-and-helm-packaging/README.md) now proposes a finite typed binding and linux/amd64 standalone default; these remain review choices distinct from this agreed materialization contract. Dockerfile-free packaging is a related capability and is not replaced by this contract.

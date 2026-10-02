# Complete OCI image indices

Experimental captured builds automatically assemble one OCI index artifact for a complete selected family of platform images. No additional packaging setting is needed. Individual images and their test/quality evidence remain in the bundle. This implements image aggregation; it does not itself provide execution, publishing, signing or release authorization. [Toolchain platform selection](toolchain-platforms.md) separately admits matching language execution images.

## Usage and prerequisites

For a Dockerfile containing `FROM scratch` and `COPY`:

```yaml
image:
  uses: docker/image
  matrix:
    platform: [linux/amd64, linux/arm64]
```

```text
oyzu run list
oyzu build image --plan
oyzu build image
oyzu inspect dist
```

Provision the existing Linux Docker toolchain and rootless BuildKit worker before building, as described in [Docker image inputs](docker-images.md). The image builder currently admits Linux amd64/arm64 assembly; foreign-platform RUN still fails without supported target execution. Windows/macOS host support does not establish native Windows/macOS container execution. Tool or image installation is not performed automatically.

`run list` retains logical builder tasks. The captured plan adds an engine-owned `assemble-index` action after the image package actions; it is not a configurable builder or a separate development task. `--plan` shows that intent after preparation without executing image builds or aggregation.

## Selection, identity and outputs

Aggregation requires at least two platform variants of one logical target, with every member selected. Corresponding image outputs must have the same unique output name and version. Different non-platform axes form separate groups: two runtime versions each receive their own complete platform index. Scalar platforms and single-platform families retain just their image. A dependency selection containing only part of a larger platform family does not invent a partial index. See [matrix selection](runtime-matrices.md).

The derived target ID uses the logical family, `oci-index`, output name and remaining axes through the existing portable-name encoding. Read IDs and paths from the plan rather than reconstructing normalized names. Each aggregate produces:

- A `kind: "oci-index"` artifact with the common image version and media type `application/vnd.oci.image.index.v1+json`.
- A file at `dist/<aggregate-id>/artifacts/<family>-<version>.oci.tar`.
- A file `digest` and `size`, plus `ociDigest` identifying the root OCI index for eventual registry publication.

The aggregate target uses builder `oyzu/oci-index`, retains non-platform variant axes and has `platform: null`. The assembly action likewise has `targetPlatform: null` and records the CLI host as its execution platform. Null means a collection of platforms, not platform-independent application output. The selection extension continues to list selected source variants; the derived aggregate is additionally present in plan/manifest targets. Its `oyzu.dev/oci-index` extension binds the family and exact required artifact IDs, producers and platforms.

These are changes to experimental v1alpha1 records. Consumers must handle derived targets and nullable aggregate platforms; ordinary image targets/actions still require concrete platforms. Synthetic [schema fixtures](../contracts/README.md) check the record shape, not successful execution.

## Assembly and failure behavior

All required image package actions must succeed before assembly. The engine checks each retained file's hash/size, native OCI closure, publication digest and platform. It writes a self-contained archive containing unchanged image manifest/config/layer bytes, deduplicated blobs and a sorted index. Stable headers and ordering make the archive independent of input enumeration order for the same image inputs.

Assembly reads archives without extraction and performs no network access or subprocess execution. It runs in the CLI process, with a 10 GiB output archive limit and at most 256 inputs. It does not claim container isolation or process-level CPU, memory or timeout enforcement. The temporary archive is verified before publication without replacing an existing destination. Assembly errors fail the action and retain the build's failure evidence under the normal [bundle lifecycle](build-bundles.md).

A failed platform blocks the aggregate; successful peer image artifacts may remain available for inspection. An index cannot silently omit the failed platform. Repair the native failure or invalid input and rebuild; do not treat a surviving peer image as a complete matrix result.

`oyzu inspect dist` checks exact index membership against the recorded required image artifacts and successful producers. It rejects a valid but incomplete index and a successful bundle missing a planned index. Inspecting the standalone OCI archive verifies its own content closure; it cannot establish the build's required platform set without the bundle. Neither form authenticates the builder or grants production publication rights.

## Verification and limits

Local Rust tests cover deterministic bytes, shared-blob deduplication, input preservation, malformed/missing content, platform/digest mismatch, duplicate platforms, no-overwrite behavior, complete-family planning, failed prerequisites and bundle membership checks. These use independently authored OCI fixtures, not native container execution. Schema checks reject missing member requirements and null platforms on ordinary targets/actions.

The Linux captured Docker suite passed actual amd64/arm64 scratch builds, their complete index, repeated archive identity, CLI inspection and a platform-specific hook failure that blocks the index in [Docker job 110864668852](https://github.com/micahlmartin/oyzu/actions/runs/37014778965/job/110864668852) at revision 1b8bd3c. This proves copy-only image assembly, not target application execution. Full EX-027/050 acceptance still requires the complete policy/ABI/cache cases and platform verification; the new producer execution checks are tracked in [toolchain platforms](toolchain-platforms.md). General platform ABI/features negotiation and registry publishing remain unfinished.

The archive follows the [OCI image index specification](https://github.com/opencontainers/image-spec/blob/v1.1.1/image-index.md): its index references platform-specific image manifests by digest.

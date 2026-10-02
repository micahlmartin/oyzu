# Docker image inputs and offline builds

The experimental Docker builder supports `scratch`, internal stages and static image references already provisioned in the executor's Docker image store. The same path captures external `COPY --from` and literal image-backed mount references reported by the native Dockerfile parser. No extra project configuration is needed:

```dockerfile
FROM alpine:3.22
COPY greeting.txt /greeting.txt
RUN test -f /etc/alpine-release
```

## Prerequisites and commands

Provision Docker with a Linux image store, the selected BuildKit user-namespace/AppArmor profile, and the builder image from `tooling/images/docker-metadata.Dockerfile`. The current image includes BuildKit 0.25.0, native metadata/image adapters and the [quality tools](docker-quality.md). Provision the declared base images separately using your organization's approved mechanism. Oyzu does not pull missing base images or read registry credentials in this profile.

```text
oyzu run list
oyzu build --plan
oyzu build
oyzu inspect dist
```

Planning includes input preparation and therefore requires the provisioned images. `oyzu run build` remains a native development command; it does not provide this captured-build boundary. Supported executor inputs currently target the selected Linux toolchain's OS/architecture. Windows and macOS hosts require a suitable Linux Docker environment; native Windows images, emulation and platform matrices remain unfinished.

## Capture, execution and evidence

1. The native parser identifies literal image requirements without executing Dockerfile commands. Unsupported dynamic references, remote `ADD`, external frontends, secret/SSH mounts and host networking fail admission.
2. The host executor inspects each provisioned image and exports its immutable config identity, so a tag change between inspection and export cannot substitute another image. It never starts that image or gives project code the host Docker socket.
3. An offline Go adapter uses go-containerregistry 0.20.6 to read the native transport archive, verify image integrity/platform and write an OCI content store. It preserves native config/layer content and media types. Images containing `ONBUILD` instructions fail because their hidden inputs are not yet part of discovery.
4. Preparation freezes the store. Plans record the requested reference, normalized native context name, config digest, manifest digest and store-tree digest. Before worker startup, the executor copies each store into private storage and verifies its planned tree identity.
5. BuildKit receives native OCI named contexts through its client session. The worker and Dockerfile `RUN` processes remain offline. Image stores are read-only worker inputs, not arbitrary host mounts or mounts inside `RUN` containers. No shared result cache is imported.

Inputs are recorded under `extensions.oyzu.dev/docker.images` in the target's `dist/dependencies/<target>.json`; `imageSource` identifies the current `provisioned-daemon` source. These identities also affect the dependency and build plan digests. The OCI manifest digest may differ from a registry's original manifest identity because a Docker save transport does not preserve every registry representation. The config identity and captured content are verified; no original registry provenance is invented.

Successful builds retain the versioned `<target>-<snapshot-version>.oci.tar` artifact and its OCI digest in `dist/manifest.json`, plus default integrity/platform JUnit and native quality results. Container integrity checks do not measure application code coverage; materialized producers retain their own test/coverage evidence. Snapshot versions still derive from source identity; changed base inputs change the plan/content digests even if that version string is unchanged.

## Failures, limits and migration

A missing image fails preparation with a provisioning diagnostic; there is no automatic internet fallback. Platform mismatch, malformed transport, config mismatch, hidden `ONBUILD`, mutated frozen stores or native worker failures block artifacts. Correct the input or provisioned toolchain, then create a new plan. Native lint/format failures retain their existing gates; adding a usable base does not waive them.

There are at most 64 distinct image references, a 10 GiB transport/captured-tree limit and 100,000 transport entries per image. Transport inspection rejects traversal, duplicate member names and transport links without extracting layer files on the host. The offline adapter is also constrained by executor resource/time limits. Images using unsupported transport forms fail explicitly. Multiple references normalizing to the same BuildKit context name currently fail as duplicate bindings rather than choosing an implicit winner.

Rebuild older custom Docker toolchain images to include `oyzu-docker-images`; images without it cannot capture bases. Prepared dependency layout version 3 adds image bindings. Scratch-only plans still work with an empty image list; old captures of unsupported external references cannot become executable merely by omitting bindings.

This is an explicitly provisioned image-input profile. Registry acquisition through approved connectors, managed source authorization, package dependencies inside images, secret brokerage, Dockerfile-free application packaging, caching and full platform matrices remain required work. The existence of an image in a local daemon is not an enterprise trust or release-eligibility assertion.

## Verification

Native image-adapter tests run from `src/builders/docker/runtime/images` with provisioned Go 1.24.13 and its checked-in module/checksum locks:

```text
go test -mod=readonly ./...
```

They generate Docker transports with the native SDK and verify OCI config/layers, repeated identities, normalization, unchanged inputs, mismatched config/platform, hidden `ONBUILD` and unsafe transport rejection. Rust checks cover required bindings, offline arguments, contained store copies and post-plan tampering. These native/unit checks do not prove Docker worker integration.

CI runs native conversion checks on Windows/macOS/Linux after compiling the CLI. The separate Linux worker job builds a provisioned-base image through the compiled CLI with a process/network/mount canary and requires repeated OCI identities. The broader captured suite exercises the checked-in EX-026 variant, reports, input evidence and missing-base failure. New isolated checks remain pending until recorded for the revision in [implementation status](../implementation-status.md).

Native API references: [BuildKit OCI named contexts](https://github.com/moby/buildkit/blob/v0.25.0/frontend/dockerui/namedcontext.go), [go-containerregistry layouts](https://github.com/google/go-containerregistry/tree/v0.20.6/pkg/v1/layout) and [Docker archive support](https://github.com/google/go-containerregistry/tree/v0.20.6/pkg/v1/tarball). The toolchain preserves dependency licenses/notices alongside the compiled adapter.

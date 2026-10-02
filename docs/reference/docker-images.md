# Docker image inputs and offline builds

The experimental Docker builder supports `scratch`, internal stages and image references already provisioned in the executor's Docker image store, including bases selected through global `ARG` defaults. The same path captures external `COPY --from` and literal image-backed mount references reported by the native Dockerfile parser. No extra project configuration is needed:

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

1. The native parser identifies image requirements without executing Dockerfile commands. Global `ARG` defaults resolve base/platform selections through BuildKit's expansion library. Unsupported dynamic mount references, remote `ADD`, external frontends, secret/SSH mounts and host networking fail admission.
2. The host executor inspects each provisioned image and exports its immutable config identity, so a tag change between inspection and export cannot substitute another image. It never starts that image or gives project code the host Docker socket.
3. An offline Go adapter uses go-containerregistry 0.20.6 to read the native transport archive, verify image integrity/platform and write an OCI content store. It preserves native config/layer content and media types. Images containing `ONBUILD` instructions fail because their hidden inputs are not yet part of discovery.
4. Preparation freezes the store. Plans record the requested reference, normalized native context name, config digest, manifest digest and store-tree digest. Before worker startup, the executor copies each store into private storage and verifies its planned tree identity.
5. BuildKit receives native OCI named contexts through its client session. The worker and Dockerfile `RUN` processes remain offline. Image stores are read-only worker inputs, not arbitrary host mounts or mounts inside `RUN` containers. No shared result cache is imported.

Inputs are recorded under `extensions.oyzu.dev/docker.images` in the target's `dist/dependencies/<target>.json`; `imageSource` identifies the current `provisioned-daemon` source. These identities also affect the dependency and build plan digests. The OCI manifest digest may differ from a registry's original manifest identity because a Docker save transport does not preserve every registry representation. The config identity and captured content are verified; no original registry provenance is invented.

The same extension's `metadata.selection` records `targetPlatform` and `sourceDateEpoch`. Preparation supplies these facts to the native adapter, and planning rejects facts that differ from the selected executor. The export epoch has one owner in the executor contract and is used both during selection and native export; it is currently `315532800` (1980-01-01 UTC).

Successful builds retain the versioned `<target>-<snapshot-version>.oci.tar` artifact and its OCI digest in `dist/manifest.json`, plus default integrity/platform JUnit and native quality results. Container integrity checks do not measure application code coverage; materialized producers retain their own test/coverage evidence. Snapshot versions still derive from source identity; changed base inputs change the plan/content digests even if that version string is unchanged.

## Global argument defaults

Image selection can use conventional Dockerfile defaults without an Oyzu configuration file:

```dockerfile
ARG REGISTRY=docker.io/library
ARG VERSION=3.22
ARG BASE=${REGISTRY}/alpine:${VERSION}
FROM ${BASE}
RUN test -f /etc/alpine-release
```

Only global declarations before the first `FROM` participate in base/platform selection. Defaults expand in declaration order using BuildKit's native quoting, escape and parameter-expansion rules. Later declarations with values replace earlier ones; declarations without a value retain an existing default. A missing value can use a native expression such as `${BASE:-alpine:3.22}`. An empty resolved base or explicit platform fails preparation. Stage-local arguments do not become global defaults, and host environment variables are never consulted.

Captured metadata records resolved stage bases/platforms and image requirements; source identity still binds the original Dockerfile. An argument resolving to `scratch` or a prior stage does not create an external image dependency. Resolved platforms remain subject to the selected executor's platform constraint.

Automatic `TARGETPLATFORM`, `TARGETOS`, `TARGETOSVERSION`, `TARGETARCH` and `TARGETVARIANT` use the captured target platform, parsed by the same pinned containerd platform library used by BuildKit. `TARGETSTAGE` names the final stage, or `default` when it is unnamed. Global declarations retain native scope and can replace these defaults. For example, `ARG BASE=registry.example/runtime:${TARGETARCH}` selects a provisioned architecture-specific image without reading the host architecture.

The executor supplies `SOURCE_DATE_EPOCH` as a build argument. A global declaration makes that value available to image selection and overrides any Dockerfile default for it. Without a global declaration, it is not part of global selection scope. A stage-local declaration can expose it to native `RUN` commands independently. These facts do not enable emulation or platform matrices; the image and selected execution platform must still match. Native lint/format policy continues to apply, including Hadolint's checks on explicit `FROM --platform` flags.

Automatic worker arguments (`BUILDPLATFORM`, `BUILDOS`, `BUILDOSVERSION`, `BUILDARCH`, `BUILDVARIANT`) still require captured worker facts. References fail with a captured-argument integration diagnostic, including fallback expressions; the target platform is not assumed to describe the worker. Public build-argument overrides and stage-local mount expansion remain unfinished.

## Failures, limits and migration

A missing image fails preparation with a provisioning diagnostic; there is no automatic internet fallback. Platform mismatch, malformed transport, config mismatch, hidden `ONBUILD`, mutated frozen stores or native worker failures block artifacts. Correct the input or provisioned toolchain, then create a new plan. Native lint/format failures retain their existing gates; adding a usable base does not waive them.

There are at most 64 distinct image references, a 10 GiB transport/captured-tree limit and 100,000 transport entries per image. Transport inspection rejects traversal, duplicate member names and transport links without extracting layer files on the host. The offline adapter is also constrained by executor resource/time limits. Images using unsupported transport forms fail explicitly. Equivalent references such as `alpine:3.22` and `docker.io/library/alpine:3.22` share one BuildKit context when their captured manifest, config and store-tree identities agree. Both requested references remain in the evidence. Conflicting identities fail planning; reference order never chooses between different images.

Rebuild custom Docker toolchain images for the current metadata adapter, which requires explicit target-platform and epoch arguments, and for `oyzu-docker-images`. Prepared dependency layout version 4 adds required selection facts to the image bindings introduced by version 3. Regenerate captures/plans made by older adapters; missing or mismatched selection facts cannot silently acquire new defaults. Scratch-only plans still work with an empty image list and explicit selection facts. No project-file migration is required.

This is an explicitly provisioned image-input profile. Registry acquisition through approved connectors, managed source authorization, package dependencies inside images, secret brokerage, Dockerfile-free application packaging, caching and full platform matrices remain required work. The existence of an image in a local daemon is not an enterprise trust or release-eligibility assertion.

## Verification

Native image-adapter tests run from `src/builders/docker/runtime/images` with provisioned Go 1.24.13 and its checked-in module/checksum locks:

```text
go test -mod=readonly ./...
```

They generate Docker transports with the native SDK and verify OCI config/layers, repeated identities, normalization, unchanged inputs, mismatched config/platform, hidden `ONBUILD` and unsafe transport rejection. Run the same command in `src/builders/docker/runtime/metadata` for native parsing, argument expansion, ignore rules and source-boundary checks. Argument tests cover composed/quoted/redeclared defaults, missing values, target architecture/variant/stage facts, epoch override/scope, host-environment independence and rejection of uncaptured worker facts. Rust checks cover selection/execution agreement, required bindings, offline arguments, contained store copies and post-plan tampering. Parsing an ARM platform in a unit test does not establish ARM execution support. These native/unit checks do not prove Docker worker integration.

CI runs native metadata/conversion checks on Windows/macOS/Linux after compiling the CLI. The separate Linux worker job builds an ARG-selected provisioned-base image through the compiled CLI with a process/network/mount canary and requires repeated OCI identities. The broader captured suite exercises the checked-in EX-026 variants, reports, input evidence, aliases, global defaults and missing-base/default failures. These captured checks passed Docker job 110767008269 at revision 8dbf1b4 in [run 36983990838](https://github.com/micahlmartin/oyzu/actions/runs/36983990838), which also passed the three-host task jobs and isolated worker job. See [implementation status](../implementation-status.md) for the exact scope and remaining work.

Native API references: [BuildKit OCI named contexts](https://github.com/moby/buildkit/blob/v0.25.0/frontend/dockerui/namedcontext.go), [go-containerregistry layouts](https://github.com/google/go-containerregistry/tree/v0.20.6/pkg/v1/layout) and [Docker archive support](https://github.com/google/go-containerregistry/tree/v0.20.6/pkg/v1/tarball). The toolchain preserves dependency licenses/notices alongside the compiled adapter.

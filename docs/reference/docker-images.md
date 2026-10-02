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

Planning includes input preparation and therefore requires the provisioned images. `oyzu run build` remains a native development command; it does not provide this captured-build boundary. Windows and macOS hosts require a suitable Linux Docker environment; native Windows images and emulation remain unfinished.

### Artifact target and worker platform

An explicit target can differ from the BuildKit worker for image assembly that does not execute target code:

```yaml
image:
  uses: docker/image
  platform: linux/arm64
```

With a Dockerfile containing only `FROM scratch` and `COPY` operations, a provisioned amd64 worker can assemble this arm64 image. Supported explicit Docker targets are currently `linux/amd64` and `linux/arm64`. Omitted `platform` retains the selected worker's platform; this increment does not introduce a new global default. Use canonical names: aliases, architecture-variant suffixes and unsupported OS/architectures fail admission. No image or emulator is installed automatically.

Preparation records the toolchain platform under `manager.platform` and the artifact target under `targetPlatform`. Plans retain tool/worker identity separately from target identity, pass the artifact target to BuildKit, and check that captured facts match the request and resolved toolchain. Base image inputs must match the artifact target. A locally provisioned base with the wrong architecture fails before export; a matching tag alone is insufficient. Materialized producers must also have the same artifact OS/architecture as their consumer. Explicit consumer platforms propagate to producer variants through materialization; this does not establish platform independence or a capable executor.

The native parser records required `targetExecution` evidence for any `RUN`, including commands in intermediate stages. Until suitable native/emulated executor admission is implemented, a Dockerfile with `RUN` requires matching worker and target platforms. A foreign-platform `RUN` fails preparation before application actions, even if the host happens to have an emulator. OCI integrity/platform assertions verify bytes and metadata; they do not execute the image or replace required application tests.

Every OCI image is checked against its artifact target during default testing, collection and bundle inspection. A valid image for the wrong architecture cannot be retained as a successful output merely because its archive hash is valid. Snapshot artifacts, JUnit and lint/read-only-format gates otherwise retain their existing behavior. Set `matrix: {platform: [linux/amd64, linux/arm64]}` instead of scalar `platform` to expand separate image builds. Each produces its own versioned OCI archive and reports. Complete selected image families additionally produce an [OCI index](oci-indices.md); see [matrix builds](runtime-matrices.md#platform-requirements-and-producer-selection) for selection, producer propagation and compatibility details.

## Capture, execution and evidence

1. The native parser identifies image requirements without executing Dockerfile commands. Global `ARG` defaults resolve base/platform selections through BuildKit's expansion library. Unsupported dynamic mount references, remote `ADD`, external frontends, secret/SSH mounts and host networking fail admission.
2. The host executor inspects each provisioned image and exports its immutable config identity, so a tag change between inspection and export cannot substitute another image. It never starts that image or gives project code the host Docker socket.
3. An offline Go adapter uses go-containerregistry 0.20.6 to read the native transport archive, verify image integrity/platform and write an OCI content store. It preserves native config/layer content and media types. Images containing `ONBUILD` instructions fail because their hidden inputs are not yet part of discovery.
4. Preparation freezes the store. Plans record the requested reference, normalized native context name, config digest, manifest digest and store-tree digest. Before worker startup, the executor copies each store into private storage and verifies its planned tree identity.
5. BuildKit receives native OCI named contexts through its client session. The worker and Dockerfile `RUN` processes remain offline. Image stores are read-only worker inputs, not arbitrary host mounts or mounts inside `RUN` containers. No shared result cache is imported.

Inputs are recorded under `extensions.oyzu.dev/docker.images` in the target's `dist/dependencies/<target>.json`; `imageSource` identifies the current `provisioned-daemon` source. These identities also affect the dependency and build plan digests. The OCI manifest digest may differ from a registry's original manifest identity because a Docker save transport does not preserve every registry representation. The config identity and captured content are verified; no original registry provenance is invented.

The same extension's `metadata.selection` records `targetPlatform` and `sourceDateEpoch`. Preparation supplies these facts to the native adapter, and planning rejects facts that differ from the selected artifact target or export epoch. The export epoch has one owner in the executor contract and is used both during selection and native export; it is currently `315532800` (1980-01-01 UTC).

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

Captured metadata records resolved stage bases/platforms and image requirements; source identity still binds the original Dockerfile. An argument resolving to `scratch` or a prior stage does not create an external image dependency. Resolved stage platforms must match the selected artifact target; mixed-platform stages remain unsupported.

Automatic `TARGETPLATFORM`, `TARGETOS`, `TARGETOSVERSION`, `TARGETARCH` and `TARGETVARIANT` use the captured target platform, parsed by the same pinned containerd platform library used by BuildKit. `TARGETSTAGE` names the final stage, or `default` when it is unnamed. Global declarations retain native scope and can replace these defaults. For example, `ARG BASE=registry.example/runtime:${TARGETARCH}` selects a provisioned architecture-specific image without reading the host architecture.

The executor supplies `SOURCE_DATE_EPOCH` as a build argument. A global declaration makes that value available to image selection and overrides any Dockerfile default for it. Without a global declaration, it is not part of global selection scope. A stage-local declaration can expose it to native `RUN` commands independently. These facts do not establish target execution capability; `RUN` still requires matching target and worker platforms. Native lint/format policy continues to apply, including Hadolint's checks on explicit `FROM --platform` flags.

Automatic worker arguments (`BUILDPLATFORM`, `BUILDOS`, `BUILDOSVERSION`, `BUILDARCH`, `BUILDVARIANT`) still require captured worker facts. References fail with a captured-argument integration diagnostic, including fallback expressions; the target platform is not assumed to describe the worker. Public build-argument overrides and stage-local mount expansion remain unfinished.

## Failures, limits and migration

A missing image fails preparation with a provisioning diagnostic; there is no automatic internet fallback. Platform mismatch, malformed transport, config mismatch, hidden `ONBUILD`, mutated frozen stores or native worker failures block artifacts. Correct the input or provisioned toolchain, then create a new plan. Native lint/format failures retain their existing gates; adding a usable base does not waive them.

There are at most 64 distinct image references, a 10 GiB transport/captured-tree limit and 100,000 transport entries per image. Transport inspection rejects traversal, duplicate member names and transport links without extracting layer files on the host. The offline adapter is also constrained by executor resource/time limits. Images using unsupported transport forms fail explicitly. Equivalent references such as `alpine:3.22` and `docker.io/library/alpine:3.22` share one BuildKit context when their captured manifest, config and store-tree identities agree. Both requested references remain in the evidence. Conflicting identities fail planning; reference order never chooses between different images.

Rebuild custom Docker toolchain images for the current metadata adapter, which requires explicit target-platform and epoch arguments, and for `oyzu-docker-images`. Prepared dependency layout version 5 requires `targetExecution` metadata and distinguishes tool execution from artifact target facts. Older metadata without that field fails rather than assuming no target execution. Regenerate captures/plans made by older adapters; missing or mismatched selection facts cannot silently acquire new defaults. Scratch-only plans still work with an empty image list and explicit selection facts. No project-file migration is required.

This is an explicitly provisioned image-input profile. Registry acquisition through approved connectors, managed source authorization, additional package-manager integrations, secret brokerage, additional Dockerfile-free application profiles, caching and full platform matrices remain required work. The existence of an image in a local daemon is not an enterprise trust or release-eligibility assertion.

## Verification

The new platform unit checks exercise arm64 image planning with an amd64 worker, mismatched capture rejection, native-only builder admission, materialization mismatch, and OCI inspection against target identity. The native parser checks target execution requirements without running project code. The Linux captured Docker suite now also requires actual arm64 OCI assembly from the EX-026 scratch fixture on an amd64 worker, JUnit/quality gates, unchanged sources, repeatable snapshot archives, and pre-action foreign `RUN` rejection. These cross-target checks passed in [Docker job 110868387899](https://github.com/micahlmartin/oyzu/actions/runs/37015880949/job/110868387899) at revision ad6cbf1; that job also verified the EX-027 and EX-050 matching-platform producer graphs described in [toolchain platforms](toolchain-platforms.md).

Native image-adapter tests run from `src/dependencies/runtime/images` with provisioned Go 1.24.13 and its checked-in module/checksum locks:

```text
go test -mod=readonly ./...
```

They generate Docker transports with the native SDK and verify OCI config/layers, repeated identities, normalization, unchanged inputs, mismatched config/platform, hidden `ONBUILD` and unsafe transport rejection. Run the same command in `src/builders/docker/runtime/metadata` for native parsing, argument expansion, ignore rules and source-boundary checks. Argument tests cover composed/quoted/redeclared defaults, missing values, target architecture/variant/stage facts, epoch override/scope, host-environment independence and rejection of uncaptured worker facts. Rust checks cover selection/execution agreement, required bindings, offline arguments, contained store copies and post-plan tampering. Parsing an ARM platform in a unit test does not establish ARM execution support. These native/unit checks do not prove Docker worker integration.

CI runs native metadata/conversion checks on Windows/macOS/Linux after compiling the CLI. The separate Linux worker job builds an ARG-selected provisioned-base image through the compiled CLI with a process/network/mount canary and requires repeated OCI identities. The broader captured suite exercises the checked-in EX-026 variants, reports, input evidence, aliases, global defaults and missing-base/default failures. These captured checks passed Docker job 110767008269 at revision 8dbf1b4 in [run 36983990838](https://github.com/micahlmartin/oyzu/actions/runs/36983990838), which also passed the three-host task jobs and isolated worker job. See [implementation status](../implementation-status.md) for the exact scope and remaining work.

Native API references: [BuildKit OCI named contexts](https://github.com/moby/buildkit/blob/v0.25.0/frontend/dockerui/namedcontext.go), [go-containerregistry layouts](https://github.com/google/go-containerregistry/tree/v0.20.6/pkg/v1/layout) and [Docker archive support](https://github.com/google/go-containerregistry/tree/v0.20.6/pkg/v1/tarball). The toolchain preserves dependency licenses/notices alongside the compiled adapter.

## Dockerfile-free packaging status

Provisioned image capture is shared through `dependencies/images`; Dockerfile parsing still determines Docker's required references. The capture path preserves existing immutable config/platform checks and OCI-store evidence. This internal relocation adds no registry fallback or configuration precedence rule.

The first application `container: true` integration is [Python application packaging](python-containers.md), whose default and override profiles passed Linux CI. It uses the internal [typed assembly boundary](../container-assembly.md) and shared captured-base acquisition. Additional runtime profiles, application-container matrices and startup smoke tests remain unfinished. Existing Dockerfile builds continue to use their captured source definition. The worker verifies the bytes actually staged for BuildKit against the planned digest, so a definition changed during copying is rejected rather than executed.


The executor's [prepared dependency-context transport](../container-assembly.md#prepared-dependency-context-transport) has native conformance verified in Linux CI. The Docker package-manager integrations are the experimental Python, Node-manager and Go-module profiles below; their end-to-end native CI results are pending. Other ecosystems, private sources and full EX-058 credential-policy acceptance remain unfinished.

## Offline pip dependency context (experimental)

For a Docker target, an external context named `dependencies` requests a prepared package store. With a single Python ecosystem and `requirements.txt`, no dependency selector is needed. Select the Docker builder explicitly when the project also looks like an application:

```yaml
image:
  uses: docker/image
```

```dockerfile
FROM python:3.13-slim-bookworm
WORKDIR /app
COPY requirements.txt .
RUN --mount=type=bind,from=dependencies,target=/dependencies \
    pip install --no-index --no-cache-dir --no-compile --find-links=/dependencies -r requirements.txt
COPY app.py .
CMD ["python", "app.py"]
```

Supply application files with the Dockerfile's ordinary COPY instructions. `--no-compile` avoids installation-time bytecode timestamps in this example; use `python -B` for build-time import checks to avoid creating timestamped bytecode afterward. Arbitrary Dockerfile commands are not automatically made reproducible. A requirements entry such as `six==1.17.0` is resolved by pip inside the exact provisioned base image, using that image's Python, pip, ABI and platform. Preparation downloads binary wheels through Oyzu's scoped public-PyPI broker. It does not add application-build, test or quality tools to this runtime store. No project code or source build runs during resolution. The subsequent Docker build remains offline and installs from the captured store; Oyzu does not rewrite RUN commands.

When other ecosystem manifests make automatic selection ambiguous, the finite override is:

```yaml
image:
  uses: docker/image
  dependencies: python/pip
```

This field is frozen build inventory, not an additional TOML precedence layer. The implemented Python providers are:

| Provider | Required native inputs | Tools already required in the consumer image |
| --- | --- | --- |
| `python/pip` | `requirements.txt` | Python and pip |
| `python/uv` | `pyproject.toml` and `uv.lock` | Python, pip and uv |
| `python/poetry` | `pyproject.toml` and `poetry.lock` | Python, pip, Poetry and its export plugin |

One matching provider can be inferred; multiple matching managers require an explicit selector, even within Python. Unknown providers and missing native manifests fail explicitly. A selector without an external `dependencies` reference also fails. A Dockerfile stage actually named `dependencies` remains a native stage, not a package-store request. Other external names retain image-reference behavior.

uv uses its [native locked, offline exporter](https://docs.astral.sh/uv/reference/cli/#uv-export) with `--no-default-groups`; Poetry uses its [native export plugin](https://github.com/python-poetry/poetry-plugin-export) restricted to the main group, with project plugins disabled. Runtime stores exclude development/default groups and optional extras. Stale locks and lock hash mismatches fail. The same native lock/source restrictions as Python preparation apply; local/path/VCS dependencies and arbitrary extra indices are not implemented by this profile. An explicit locked provider consumes its own lock, not a neighboring requirements file.

All three providers include a generated `requirements.txt` beside the resolved wheels. It pins the complete selected closure with SHA-256 hashes. This is a projection of the already-resolved native inputs; it does not replace the source lock or perform another dependency resolution. For a uv/Poetry project, an offline Dockerfile step can consume it without copying or exporting the project lock itself:

```dockerfile
RUN --mount=type=bind,from=dependencies,target=/dependencies \
    pip install --no-index --no-deps --require-hashes --no-cache-dir --no-compile --find-links=/dependencies -r /dependencies/requirements.txt
```

uv's native `uv pip install` can consume the same hashed requirements. This is not transparent support for arbitrary `uv sync` or `poetry install` commands/cache layouts. Missing native tools in the base fail preparation; Oyzu never installs them. Native tests currently provision uv 0.12.21 with Python 3.12, and Poetry 2.5.1/export 1.10.1 with Python 3.12. Other tool versions require qualification.

The base image must already exist locally and provide Python 3.11 or later with pip; the first native fixture uses Python 3.13. The worker and artifact platform must match for package preparation, even for a Dockerfile that only copies store files. Consumers using earlier named stages resolve to their original base. Multiple distinct consuming bases, `scratch` consumers and incompatible platforms fail rather than sharing an unchecked closure. Preparation resolves the already-captured immutable base config, not a tag that may have moved. Commands that subsequently change the interpreter or libraries in a stage remain the Dockerfile author's responsibility; the initial runtime binding does not prove compatibility after arbitrary stage mutations.

The prepared dependency record retains wheel identities, digests, source/lock identities, actual interpreter and manager versions, immutable runtime config and a separate store-tree digest. Only the wheel subtree and generated hashed requirements enter BuildKit's private named context; broker/control state stays outside it. The context is read-only by default for the shown native bind mount. Build output remains the normal versioned OCI archive with integrity/platform JUnit and Docker quality gates. Integrity evidence is not a release authorization or proof that a package's contents are trustworthy.

The initial profile uses the existing public PyPI and Python-hosted-files routes. Direct URL requirements, alternate-index directives and source distributions are rejected. Native requirement constraints/hashes retain their existing pip validation. This preparation needs upstream availability; there is no persistent offline acquisition cache yet. Managed acquisition and enforced connector routes fail closed until approved connector bindings exist. No registry token is supplied to the image, build arguments or Dockerfile. This does not yet implement the private-registry canary and all-manager credential requirements of EX-058. Remote syntax frontends and secret mounts remain unsupported by the captured Docker profile.

On failure, inspect the retained diagnostic and native logs, correct the manifest/provider or provisioned runtime, and start a new build. Preparation failures produce no action artifacts; failed offline RUN commands block packaging. The Linux Docker CI group provisions the three Python profiles and exercises native resolution/install/import, exact package/runtime evidence, versioned output, repeatability, implicit tasks, ambiguity/explicit selection, excluded development groups, lock-hash rejection and forbidden build-time network fetching. Its first native results remain pending. Provisioned Windows uv/Poetry probes have verified native export, unchanged locks, hashed offline installation/import, wheel-tamper rejection and stale-lock rejection; these narrower probes do not establish Docker isolation or container output.

## Offline npm dependency context (experimental)

`node/npm` exports verified registry tarballs using the Node builder's existing native locked acquisition. A Docker target with `package.json` and a v2/v3 `package-lock.json` or `npm-shrinkwrap.json` can infer this provider. Shrinkwrap takes precedence, as it does in npm. Multiple ecosystems require `dependencies: node/npm` in the target's build declaration. The shared runtime, platform, policy and context rules above also apply; the captured consumer base must already contain Node and npm. A declared `packageManager` must match that provisioned npm version, and native npm validates engine constraints. The tested profile uses npm 11.11.0 with Node 22 in CI fixtures and Node 24.14.1 in a Windows native probe.

For the provisioned CI image, a complete Dockerfile example is:

```dockerfile
FROM oyzu-toolchain/node:npm11.11.0-node22
WORKDIR /app
COPY package.json package-lock.json ./
RUN --mount=type=bind,from=dependencies,target=/dependencies \
    for archive in /dependencies/*.tgz; do npm cache add "$archive" --offline --cache /tmp/npm-cache --ignore-scripts; done \
    && npm ci --offline --cache /tmp/npm-cache --ignore-scripts --omit=dev --no-audit --no-fund \
    && rm -rf /tmp/npm-cache
COPY app.js .
CMD ["node", "app.js"]
```

Use your already-provisioned compatible base instead of the CI image as appropriate. Copy all required workspace manifests/sources for a workspace install, and copy `npm-shrinkwrap.json` instead when using shrinkwrap. The shown glob expects at least one captured registry package; dependency-free projects can run native offline npm without seeding a store. No generated Dockerfile or automatic rewrite of RUN commands is involved.

The store contains only `<sha256>.tgz` files. Preparation fetches all locked registry tarballs through the scoped public npm broker, validates SHA-512 lock integrity, and invokes native npm offline with lifecycle scripts disabled to reject stale locks and incompatible installations. Local workspace links stay in the source graph; native npm owns dependency resolution and installation layout. Development and optional packages are captured too, with their existing evidence purposes; this is an all-lock store rather than a runtime-only pruning resolver. The shown consumer decides to omit development installation through native `--omit=dev`. Acquisition does not execute project lifecycle scripts; the example also disables them during the offline build. Packages requiring lifecycle compilation need an explicitly provisioned compiler and a deliberate offline Dockerfile command.

Only tarballs enter the named context. npm's temporary cache, logs, user/global configuration, acquisition workspace and broker state are not exported. The consumer seeds its own disposable cache and removes it in the same RUN instruction, avoiding both cache log timestamps and an extra cache layer. Native npm verifies the lock again during installation. Package names, versions, purposes, content hashes, lock/source identity, observed Node/npm versions and immutable base identity remain in `dist/dependencies/<target>.json`; the normal versioned OCI artifact, JUnit and Docker quality checks remain mandatory. This does not promise reproducibility for arbitrary lifecycle scripts or other Dockerfile commands.

Missing/stale locks, unsupported lock versions, integrity failures, incompatible engines or a mismatched declared package manager fail preparation. Correct the native inputs or provisioned base and rebuild; preparation failures emit no actions/artifacts. The current public route is `https://registry.npmjs.org/`; private registries, VCS/direct non-registry sources, managed connector authorization and npm-specific credential canaries are not implemented. There is no persistent acquisition cache or internet fallback during execution.

`tooling/test-npm-context.py` passed on Windows with actual npm 11.11.0/Node 24.14.1. It checks broker-transported product acquisition, script suppression, tarball-only export, two fresh offline cache/install operations, transitive runtime packages, omitted development installation, unchanged locks/store and rejected integrity. Its fixture transport does not prove the production broker's network isolation. The registered Linux `docker-npm-context` group additionally requires compiled-CLI preparation, actual offline RUN/import, exact resulting image files, package/runtime evidence, implicit tasks, OCI/JUnit/quality output, repeated identities and integrity rejection before actions. Its native result is pending; this is not full EX-058 acceptance.

## Offline Yarn Classic dependency context (experimental)

`node/yarn` exports a native offline mirror after the existing Yarn acquisition adapter validates the frozen dependency graph. With `package.json` and `yarn.lock`, it is inferred for a Docker target when there is no competing manager/ecosystem. Multiple native locks require an explicit `dependencies: node/yarn` selector. A modern Yarn lock is not silently treated as Classic. The consumer image must already contain Node, Yarn Classic 1.22.22 and the adapter's `@yarnpkg/lockfile` 1.1.0 parser at `/opt/oyzu-yarn/node_modules/@yarnpkg/lockfile` (as in the provided toolchain image). The native adapter also accepts its existing `OYZU_YARN_LOCKFILE` image-provisioning path. Oyzu installs none of these tools during preparation.

For the provisioned CI image:

```dockerfile
FROM oyzu-toolchain/node:yarn1.22.22-node22
WORKDIR /app
COPY package.json yarn.lock ./
RUN --mount=type=bind,from=dependencies,target=/dependencies \
    printf 'yarn-offline-mirror "/dependencies"\ndisable-self-update-check true\n' >/tmp/oyzu.yarnrc \
    && yarn install --offline --non-interactive --frozen-lockfile --ignore-scripts --use-yarnrc /tmp/oyzu.yarnrc --cache-folder /tmp/yarn-cache \
    && yarn cache clean --offline --non-interactive --cache-folder /tmp/yarn-cache \
    && rm -rf /tmp/yarn-cache /tmp/oyzu.yarnrc
COPY app.js .
CMD ["node", "app.js"]
```

Only the verified mirror archives enter `dependencies`; temporary Yarn caches, configuration, broker state and inventory files do not. The ephemeral consumer configuration points at that read-only context, and cleanup occurs in the same RUN instruction. Preparation uses the consumer's exact immutable Node/Yarn runtime, validates native package-manager/engine constraints, and keeps lifecycle scripts disabled. Native Yarn performs dependency selection, including supported registry-only `resolutions`; the adapter rejects a changed native graph before exporting the mirror. Source locks remain unchanged. Like its application-build adapter, Yarn captures all locked inputs and conservatively records their purpose as `build`, without claiming a resolved runtime-only inventory.

The admitted profile supports a single project, Classic v1 locks, SHA-512 integrity, transitive and scoped registry packages, and registry version-range resolutions. Acquisition uses the existing public npm/Yarn routes. Workspaces, custom `.yarnrc`/`.yarnrc.yml`/`.npmrc`, path/VCS dependencies, modern Yarn, private registries and managed connector authorization remain unsupported. The shown configuration is generated inside the private RUN filesystem, not checked into the source project. Missing tools, unsupported input profiles, stale locks, altered integrity or changed runtime identity fail before artifacts; correct the inputs/provisioned base and rebuild. Offline execution cannot fill a missing store from the internet, and there is no persistent acquisition cache yet.

The mirror digest, observed manager/runtime, original source/lock identities and verified package identities remain in the Docker dependency evidence; ordinary snapshot OCI output, required JUnit and quality gates are unchanged. Windows native checks using Yarn 1.22.22/Node 24.14.1 passed ordinary and selective-resolution profiles, stable repeated captures, scoped/transitive mirror export, fresh-cache offline installation/tests, lifecycle suppression, unchanged locks/mirrors and corrupt mirror rejection. Existing pnpm patch capture/replay also passed after the shared lifecycle change. These checks use a fixture spool and do not establish Docker isolation. The registered Linux `docker-yarn-context` group requires real compiled-CLI preparation, offline install/import, exact image contents, evidence, repeatability and altered-lock rejection; its native result remains pending.

## Offline pnpm dependency context (experimental)

`node/pnpm` prepares a native pnpm store in the immutable consumer image, using the Node adapter's existing brokered registry archives and frozen-lock validation. `package.json` and a single-project v9 `pnpm-lock.yaml` select it when unambiguous; otherwise use `dependencies: node/pnpm`. The image must already provide Node, pnpm **10.11.0** and `yaml` 2.8.1 at `/opt/oyzu-pnpm/node_modules/yaml`, as in the provisioned toolchain. The adapter's existing `OYZU_PNPM_YAML` image-provisioning path is also accepted. The exported store format is deliberately qualified against this exact pnpm version; other versions fail context export pending qualification rather than borrowing cache semantics silently.

```dockerfile
FROM oyzu-toolchain/node:pnpm10.11.0-node22
WORKDIR /app
COPY package.json pnpm-lock.yaml ./
RUN --mount=type=bind,from=dependencies,target=/dependencies \
    cp -r /dependencies /tmp/pnpm-store \
    && pnpm --config.manage-package-manager-versions=false install --offline --frozen-lockfile --ignore-scripts --ignore-pnpmfile --config.verify-store-integrity=true --config.side-effects-cache=false --store-dir /tmp/pnpm-store \
    && rm -rf /tmp/pnpm-store
COPY app.js .
CMD ["node", "app.js"]
```

The named context stays read-only. Copying its store gives pnpm a private writable location for its native verification metadata; remove it in the same RUN instruction. Native package imports remain usable after that store is removed. This uses pnpm's [offline installation semantics](https://pnpm.io/10.x/cli/fetch), not an Oyzu resolver or a renamed-tarball cache. Adding local tarballs with `pnpm store add` does not create the registry identities needed by this qualified frozen-install profile.

Only the native `v10/files` content and `v10/index` records are exported. The adapter resets each index file's `checkedAt` verification hint to zero and serializes index keys deterministically. Package bytes, native integrity values, modes, sizes and package identity remain unchanged; zero prevents relying on acquisition-time freshness shortcuts after copying. The pinned native consumer rechecks the copied content with store-integrity verification enabled. The exporter rejects links, special files, unknown store paths/index fields and side-effect caches, with the existing 100,000-file/10-GiB capture limits and a 16-MiB per-index limit. Empty locked graphs can export an empty store. User configuration, native log/cache state outside this content store, temporary paths and broker state are not included.

The profile retains existing single-project registry and source-patch support. For a patched project, copy `pnpm-workspace.yaml` and `patches/` before installation, or use the supported `package.json` patch declaration. The native frozen install applies and verifies patches; dependency evidence retains original package integrity and source-patch digests. Acquisition has lifecycle scripts and side-effect caching disabled. All locked registry inputs are captured and conservatively classified as build inputs, without claiming runtime-only pruning. Workspaces, `.npmrc`, `.pnpmfile.cjs`, unsupported native overrides, direct/path/VCS dependencies, other pnpm versions and managed/private connectors remain explicit gaps.

Stale locks, altered archive integrity, missing native tools or unqualified store metadata fail preparation and produce no action artifacts. Correct the inputs or provisioned toolchain and rebuild. Build actions have no registry access: an incomplete/corrupt offline store fails instead of being replenished from the internet. Existing context/base identity checks, versioned OCI artifacts, JUnit and quality gates apply. No credential is placed in the store by Oyzu; package content integrity is not provenance or release authorization.

Windows native checks using pnpm 10.11.0/Node 24.14.1 passed ordinary, patched and empty-graph preparation, repeated export identities, fresh offline consumption, tests after temporary-store removal, unchanged source locks and rejection of tampered store content. A Yarn selective-resolution mirror check also passed after the shared export-callback update. Both generated pnpm Dockerfiles passed native Hadolint and read-only dockerfmt checks. The registered Linux `docker-pnpm-context` group requires actual compiled-CLI capture/builds for ordinary and patched projects, exact image files/patch evidence, offline imports, snapshot OCI/JUnit/quality, repeatability and pre-action lock-integrity rejection. Those container results remain pending; the host probes use a fixture spool and do not qualify the production broker or full EX-058.

## Offline Go module context (experimental)

`go/modules` exposes the Go builder's native, checksum-verified module cache through `dependencies`. A Docker target containing `go.mod` or `go.work` can infer this provider; mixed ecosystems require `dependencies: go/modules`. Native metadata resolves contained workspace members, local replacements and test imports using the original source/checksum files. The existing public Go proxy broker obtains required module inputs, and native Go verifies `go.sum`. Preparation must leave manifests and checksum files unchanged.

The consumer base must already contain Go and a POSIX shell; projects using cgo also require the configured C compiler. The CI profile uses the standard `golang:1.24-bookworm` image. Context preparation compiles the owned metadata helper with that provisioned Go compiler using only the standard library. It does **not** require `oyzu-go-modulezip`, project snapshot version projection or module artifact packaging. Those remain responsibilities of ordinary `go/app`/`go/library` builds. Toolchain auto-download, VCS access and user Go configuration are disabled during preparation.

For a pure-Go command with a complete `go.sum`:

```dockerfile
FROM golang:1.24-bookworm AS build
WORKDIR /src
COPY . .
RUN --mount=type=bind,from=dependencies,target=/dependencies \
    GOMODCACHE=/dependencies GOPROXY=off GOSUMDB=off GOTOOLCHAIN=local CGO_ENABLED=0 go build -mod=readonly -trimpath -buildvcs=false -o /app .
FROM scratch
COPY --from=build /app /app
ENTRYPOINT ["/app"]
```

The complete prepared module subtree is mounted read-only. The build uses native cache lookup with registry access disabled; it does not receive a broker, credentials or Go acquisition metadata as files. Native metadata, observed Go version/platform, lock/source digests and used module archive identities remain in the dependency evidence. The context's module-artifact list is empty because dependency preparation does not publish the project's own module. Compiler output/cache stays in the private build stage, and the final scratch stage contains only the copied binary. Use a suitable runtime instead of scratch for cgo or other runtime-library requirements; arbitrary Dockerfile commands remain responsible for compatible compilation and reproducibility.

Missing checksums, stale manifests, escaping workspace/replacement paths, failed native metadata or incompatible tools fail preparation before action artifacts. Update and commit native metadata or provision the correct image, then rebuild. The shared provider policy/platform rules still apply, including managed acquisition rejection until approved connector bindings exist. Private module routes, direct VCS sources, checksum-database service integration and persistent acquisition caches remain unfinished. Current verification uses committed `go.sum`; it does not claim independent checksum-database attestation or production release authority.

Windows Go 1.24.13 native checks passed existing workspace/cgo and checksum cases, plus two detached module-store copies compiled offline into identical binaries that both ran with expected output. Source and store bytes remained unchanged, and no extra fixture broker requests occurred during replay. The exact Dockerfile passed native Hadolint/read-only dockerfmt. The Linux `docker-go-context` group requires real compiled-CLI preparation in a standard Go image, read-only cache compilation and execution in the build stage, exact final binary contents, snapshot OCI/JUnit/quality, repeatability and checksum rejection. Container results remain pending. Docker integrity JUnit does not replace source tests or coverage; use a separately built Go producer when those language-builder reports are required. The check does not claim it starts the final image.

---
id: OEP-0018
title: Container packaging materialization and Helm composition
status: draft
implementation: in-progress
updated: 2026-10-01
authors: [micahlmartin]
reviewers: []
requires: [OEP-0006, OEP-0007, OEP-0012, OEP-0014, OEP-0017]
tracking-issue: null
---

# Container packaging materialization and Helm composition

> Implementation specification for review. Requirements and v1alpha1 choices remain draft; no maintainer acceptance, implemented support or verified security is implied.


## Developer experience and artifact selection

Containers are optional packaging capabilities of application builders, or explicit `docker/image` targets for existing Dockerfiles. A project with an unambiguous Dockerfile discovers an image target. A minimal `api: {uses: python/app}` selects application intent without mandatory tests/output sections; it does not by itself assert every Python application must be a container. Add only `container: true` when container output cannot be inferred from native project intent. This field is a proposed v1alpha1 choice, not a retroactive change to the agreed examples.

```yaml
api:
  uses: python/app
  container: true
```

The convenience path discovers the application entrypoint from supported native metadata. If ambiguous, `entrypoint` is an argv array on the container option object; `container: {entrypoint: [python, -m, service]}` is the narrow override. Other initial optional container object fields are `base` (approved image reference resolved to a digest), `user` (numeric UID:GID) and `workdir` (absolute contained image path). No generated Dockerfile must be checked in. Runtime environment secrets and deployment mechanics are out of scope.

The builder selects a versioned runtime profile: base identity, runtime compatibility, nonroot UID/GID (default 65532:65532 when compatible), working directory `/app`, entrypoint and required runtime files. Do not invent ports/health checks from package names. Default bases are maintained, integrity-pinned builder descriptor data and can be replaced/constrained centrally. Missing an approved compatible profile fails rather than silently selecting a public latest image. Generated profile details appear in the plan and bundle.

## Application assembly

Python packages install from captured wheels into a runtime environment matching the target interpreter path/ABI; do not copy the host virtualenv. Node packages include the built app plus production runtime dependency closure using the selected manager's supported deploy/layout mechanism; retain PnP where appropriate. Go/Rust copy exact tested producer binaries plus necessary runtime libraries/CA roots. Java copies selected JAR/WAR/application distribution with a compatible JRE/server runtime profile; ambiguous launch semantics require native metadata or an entrypoint override. A static binary may use scratch only when its runtime needs are established, not because it was written in Go.

Build/test application outputs once and feed their exact digests into the packaging action. Packaging never recompiles the application as an undocumented second producer. An image smoke test is a separate action tied to the image digest and actual execution platform. A target-platform test requirement cannot be satisfied by merely cross-compiling an image's binary.

## Consumer materialization

Retain the agreed `materialize: [{from: api, artifact: server, to: bin/server}]` record; omit artifact only for a unique primary output. `from` implies an edge. A file lands at an exact file path; directory contents land under to. No dist scraping, source mutation, implicit archive extraction or duplicate ordering edge is needed. All mappings resolve before execution to logical output contracts and compatibility requirements; actual bytes are checked just before consumption.

Build the source context from the captured tree and Docker ignore rules first, then validate explicit destinations against all retained source paths and other materializations. Reject overlaps, portable case aliases, parent escapes, escaping links and source collisions. Explicit materialized inputs remain present regardless of ordinary source ignore rules; an adapter that cannot ensure this must fail. Preserve executable metadata. Consumers get private workspaces and cannot mutate the producer CAS. Remote placement is allowed without local download.

## Platforms and multi-image outputs

Initial standalone default target is proposed linux/amd64 regardless of invocation host. Explicit platform and managed defaults can select another compatible target. Report the resolved default before execution. `matrix.platform` produces independent variant contexts; runtime artifact requirements propagate backward to producer variants. Build tools execute on the executor architecture. Check target OS/arch plus libc, CPU feature floor, runtime and native library requirements. Conflicts with explicit producer constraints fail during planning.

Each platform image yields an OCI manifest digest; the group yields an OCI image index referencing complete successful variants. Do not silently publish a partial index if one required variant failed. The bundle exports an OCI layout with referenced blobs and index; registry publication is a later operation. Content reuse across platforms requires established independence and equal relevant input digests. Emulated tests record emulation and actual target execution; unsupported emulation/native test capability fails a mandatory target test.

## Custom Dockerfiles and dependency contexts

Use BuildKit with approved pinned frontend identity. FROM references, external COPY --from images and supported remote inputs must resolve to approved captured digests during preparation. Reject undeclared remote ADD, SSH forwarding, host networking, privileged entitlements, host sockets and unbounded external contexts. Docker's build networking flag alone does not constrain base/frontend acquisition; route and capture those separately. An existing Dockerfile remains authoritative within these limits; do not silently rewrite arbitrary RUN shell code.

The initial custom dependency integration adds one optional `dependencies` builder field identifying a supported manager (`python/uv`, `python/pip`, `python/poetry`, `node/npm`, `node/pnpm`, `node/yarn`, `java/maven`, `java/gradle`, `java/ivy`, `go/modules`, `rust/cargo`, `helm`, `os/apt`, `os/apk`). Its native manifest root is the Docker target's path. When exactly one supported manager is inferable and the Dockerfile references named context `dependencies`, omit the field. Ambiguity requires this one selector, not an authentication block. The schema registers the field only for docker/image.

The adapter binds a credential-free named context `dependencies` with a documented manager-specific layout (Python wheels at context root; other store layouts versioned by adapter). A standard bind mount consumes it, as in EX-058. This is distinct from copying a producer artifact via materialize; no fictional dependency target appears in user YAML. Capture the binding/layout/digest in the plan. The adapter must prove offline native consumption before advertising support; unsupported custom package-manager commands fail with the required integration instructions.

apt/apk integrations prepare repository metadata and exact archive closure. Arbitrary existing apt-get update/apk add commands do not become hermetic automatically. Provide supported offline installation instructions using the prepared context or generated convenience packaging, and reject unsupported network-dependent Dockerfiles. Automatic shell rewriting is explicitly excluded. These controls resolve the proposed shape from EX-057/058 without claiming their implementations exist.

## Credentials and engine cache

Upstream registry credentials stay in the broker or container engine's separately scoped acquisition session, never ARG/ENV/COPY or application RUN. BuildKit secret mounts are only an explicit approved escape hatch for nonstandard secret actions; mount contents can still be copied or logged. Such actions cannot inherit the normal credential-exclusion guarantee. Their cache/export rules must be separately justified. Inspect full layers, image config/history, frontend provenance and exported intermediate cache, not just the final filesystem.

Disable unvalidated cross-run BuildKit result-cache reuse when it cannot provide the required producer evidence. Oyzu may use its own verified whole-image action cache while BuildKit runs with private per-run stores. Later shared BuildKit cache import requires a provenance-preserving adapter; a BuildKit cache hit is not automatically equivalent to an Oyzu action result. Docker documents that secret values do not invalidate build cache; therefore rotating a token is not a computation identity or a safe cache strategy. [Docker cache invalidation](https://docs.docker.com/build/cache/invalidation/)

## Helm packaging and typed image binding

Helm discovers Chart.yaml and locked dependencies, prepares HTTP/OCI charts, runs supported lint/template checks using captured values/capabilities, and packages a chart. Deployment, cluster access and release installation are excluded. Values required for meaningful rendering must be provided natively; do not invent a cluster. Record Kubernetes/API capability fixtures as inputs. Packaging preserves appVersion versus chart version as separate native fields.

For image/chart composition, use one proposed finite typed binding:

```yaml
chart:
  uses: helm/chart
  path: chart
  bindings:
    - from: image
      artifact: image
      type: image-reference
      repository: /image/repository
      digest: /image/digest
```

The destination paths are JSON Pointers into the chart values map; no expressions/templates run in the binding resolver. The image artifact selects its planned registry repository plus actual OCI manifest/index digest. This creates a dependency and injects values only into an isolated chart packaging/rendering copy. The chart's native templates must consume the digest field; adapters cannot assume every chart supports digest references. Missing/nonstring/conflicting destinations fail. No tag-only substitute is permitted. Image publication need not happen during build; planning must know the approved eventual repository, and later publishing to a different repository requires a rebuilt/rebound chart rather than silently rewriting a signed artifact.

The producer image's `image` output denotes its index for a platform matrix or single manifest otherwise. Select platform-independent metadata when building a chart, not an arbitrary binary variant. Default publication dependency order is image verified at destination → chart, with separate receipts; there is no cross-registry atomicity or implied deployment.

## Acceptance scenarios

- PACK-01: Optional Dockerfile-free packaging consumes exact prepared application output once and infers tests/reports without boilerplate.
- PACK-02: Materialization preserves stable paths and metadata while rejecting collisions, escapes and incompatible variants.
- PACK-03: Multi-platform images retain matching producer/test evidence and never silently emit a partial required index.
- PACK-04: Custom Dockerfiles consume approved captured inputs; base/frontend/network/secret paths cannot bypass acquisition controls.
- PACK-05: Image layers, metadata, logs and intermediate caches contain no upstream credential under the normal packaging path.
- PACK-06: Helm digest bindings alter only isolated values, create graph edges and fail incompatible charts/destination changes.
- PACK-07: Container engine cache reuse preserves producer evidence or is disabled; it never launders local-origin output.

## Rollout and open implementation gates

Implement Go binary materialization first, Python convenience packaging next, then custom dependency contexts and native-runtime variants, followed by multi-platform index and Helm binding. Apply the same contract to all supported language adapters; unsupported profiles fail explicitly until their tests pass. Validate pinned runtime profiles, BuildKit transport/frontend integration, native-store mounts and OCI interoperability before release. The proposed container/dependency/binding field spellings and stable standalone platform default require maintainer review; the associated draft schema makes them concrete to review rather than leaving implementation to guess.

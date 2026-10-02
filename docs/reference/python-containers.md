# Python application containers (experimental)

The captured build path accepts `container: true` for a supported `python/app`. It packages the already-tested pure-Python application archive into an OCI image without a source Dockerfile or second application build. The first Linux run produced the native image but failed plan-schema validation because its report declaration omitted required fields. That defect is corrected; a fresh full acceptance result remains pending. It does not yet provide application startup smoke tests, additional language runtime profiles or full EX-012 acceptance.

```yaml
api:
  uses: python/app
  container: true
```

Run `oyzu run list`, `oyzu build` and `oyzu inspect dist` as usual. Native application build/test/lint/read-only-format tasks remain visible and retain their existing overrides and hooks. Image build, assertion and package actions appear under a derived `api-container` target in the plan and manifest. These derived actions are internal build actions; direct `oyzu run` invocation and overrides/hooks for them are not implemented. A source target colliding with the derived name fails rather than being overwritten.

## Prerequisites and runtime contract

Provide the native Python toolchain/dependencies required by [Python applications](python-applications.md), plus `python:3.12-slim-bookworm` and `oyzu-toolchain/docker:buildkit0.25.0` in the local Docker image store. The latter comes from `tooling/images/docker-metadata.Dockerfile`. Provision the appropriate AppArmor profile where required. The CLI does not pull missing images, install Docker or change host security settings. Execution uses Linux containers; complete native acceptance of this integration is pending.

The language-owned `python-pure-zip-3.12-v1` profile requires captured CPython 3.12 preparation facts and a pure, ZIP-compatible application artifact. Preparation probes the provisioned runtime's implementation/minor version without network access, captures its immutable image identity and OCI content, and rejects a changed image between probing and capture. This is integrity and compatibility evidence, not registry authentication or production authority. Base bytes must be provisioned through an appropriate trusted process. Pinned base approval and centrally configurable runtime profiles remain further work.

The default runtime layout is: `/app/application.pyz`, numeric user/group `65532:65532`, working directory `/app`, and exec-form entrypoint `["python", "/app/application.pyz"]`. Inherited CMD is cleared. No ports, health checks, server lifecycle, deployment settings or secrets are inferred. No build tools or project dependencies are installed inside the image assembly action; the archive already contains its captured pure-Python runtime closure.

## Focused overrides

Use an object only when the defaults need to change. The following example requires the named base to have been provisioned locally with a compatible CPython 3.12 runtime:

```yaml
api:
  uses: python/app
  container:
    base: registry.example/python-runtime:3.12
    user: "1000:1001"
    workdir: /srv/api
    entrypoint: [python, /app/application.pyz]
```

Each field is optional; `{}` has the same packaging defaults as `true`. `base` is a literal image reference (up to 512 bytes), resolved to immutable local content and subjected to the same platform/CPython probe and captured-base admission as the default. Missing or incompatible bases fail; no pull or public fallback occurs. `user` is numeric UID:GID, each within the unsigned 32-bit range; named accounts and negative identities fail. The same identity owns the copied archive. `workdir` is an absolute literal image path (up to 1,024 bytes, ASCII letters/digits and `/._-`, without parent escapes). It does not relocate the archive from `/app/application.pyz`. `entrypoint` is an exec-form array of 1-64 nonempty literal arguments (up to 4,096 bytes each, no control characters). No shell or variable expansion is implied. Unknown keys, wrong types and explicit null fields fail configuration validation.

Overrides are target intent in the frozen `build.yaml` inventory, not additional TOML precedence layers. They are bound by the source/plan identity, while execution settings still use the resolved configuration snapshot. The selected native application entrypoint and ZIP payload must already be unambiguous; changing the image entrypoint does not resolve multiple native console scripts. A custom argv is declared launch metadata, not evidence that its executable exists or that the service starts. The existing runtime compatibility probe always runs, even with a custom entrypoint. Base approval and mandatory runtime-user policy remain future integrations; these overrides confer no production authority.

## Artifacts, checks and failure behavior

The original `application`, `wheel` and `sdist` artifacts remain available for a distribution application; requirements-only applications retain their application artifact. The derived target adds an `image` artifact at a manifest-declared path such as `api-container/artifacts/api-0.1.0-dev.g<source>.oci.tar`. OCI image versions use the existing semver snapshot spelling, independently of Python's PEP 440 wheel/archive version. The manifest records the file digest and OCI publication digest.

Packaging starts after the application package action, which already depends on its tests, required reports and quality gates. Shared materialization checks and copies the exact retained archive into an otherwise empty private image context and links the producer's test/coverage evidence. Generated image definitions never enter the checkout. Image content/platform assertions produce a separate JUnit report; image packaging assertions do not fabricate application coverage. A failed application prerequisite blocks image assembly; failed OCI assertions block image publication into the bundle. Inspect retained logs and reports, fix the native error and rebuild.

The normal configuration engine owns precedence and policy checks. Docker tool eligibility is checked alongside the application before preparation. `docker.apparmorProfile` comes from the resolved owner snapshot and is frozen into the image action. No execution-time environment override replaces it. Profile names do not grant signing/publication rights. Managed application acquisition remains subject to the existing connector-binding restrictions; this feature introduces no managed download fallback.

Boolean configurations retain their existing meaning. Previously rejected option objects are now supported with the finite contract above. No generated Dockerfile or extra project task is required.

## Current limits and verification

`container: false` disables the option. Legacy/native Python application layouts, application-container matrices, other language runtime profiles and image startup smoke execution remain unimplemented and must not be inferred from this path. Unsupported options/profiles fail explicitly. Application startup behavior is not established merely by passing archive tests or OCI assertions. Discovery does not infer container intent for every Python application.

The Python captured CI suite runs the authored `python-api/variants/container.build.yaml` and `container-overrides.build.yaml` cases. It requires exact application bytes in the image, a compatible captured runtime, nonroot launch metadata, application reports and image JUnit, unchanged sources, repeatable plan/artifact identities, a failed-test case that blocks image generation, and incompatible custom-runtime rejection. The override case checks actual image metadata, exact unchanged application bytes, captured custom-base identity and repeatability. CI provisions tools before invoking the compiled CLI. Local Rust profile tests and existing shared acquisition/worker checks establish narrower contracts; consult [implementation status](../implementation-status.md) for actual native results.

The retained image from [Python job 110889424053](https://github.com/micahlmartin/oyzu/actions/runs/37021975506/job/110889424053) at 9b84d6a contains the exact application archive and expected default launch metadata. The build retained application JUnit/coverage and OCI assertion JUnit. The job failed on missing `required` and `subject` fields in the generated plan report declaration; repeated-build and failed-test acceptance steps did not run. Report intents now use the same typed contract as native and custom tasks, with a regression against the checked-in plan schema. These partial native results do not substitute for the pending complete scenario run.

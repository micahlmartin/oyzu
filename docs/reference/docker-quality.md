# Dockerfile linting and formatting

Dockerfile discovery now exposes native lint, read-only format checks and explicit formatting without an Oyzu configuration file. The defaults are Hadolint 2.15.1 and dockerfmt 0.5.4. They complement the existing isolated image build and OCI validation; neither tool builds or runs the application.

## Tasks and behavior

```text
oyzu run list
oyzu run lint
oyzu run format-check
oyzu run format
oyzu build
```

`lint` runs `hadolint --no-color Dockerfile`. Native rules, severities, configuration and inline suppressions determine its exit status. `format-check` runs dockerfmt with `--check` and does not change source. Only the explicitly requested `format` task uses `--write`; it is marked mutating and is not a build stage. With no target-local `.editorconfig`, Oyzu adds `--newline` so its default formatting retains a final newline. With that file present, native dockerfmt configuration and defaults determine formatting, including the EOF convention.

These tasks appear even when tools have not yet been provisioned. Static discovery records native configuration evidence or a fallback selection without executing tools. Missing binaries fail execution with installation guidance; they are not downloaded or silently skipped. Task overrides and pre/post hooks use the existing shared task engine. A replacement task owns its behavior and is not automatically another native checker.

Hadolint interprets target-local `.hadolint.yaml`/`.hadolint.yml`, including ignored rules and severity settings. Dockerfmt interprets target-local `.editorconfig`. Oyzu does not parse either into a new configuration language. Invalid native configuration remains a native failure. Development execution can also observe ambient native configuration; captured execution has no host configuration mounts. Parent configuration outside the captured target is not currently imported, so keep required configuration inside the target for matching captured behavior.

In a captured build, lint and format-check are required gates before final artifact collection. Failure retains native action diagnostics and blocks the snapshot OCI artifact. The captured source and live checkout are unchanged by those checks. Repair lint failures, run explicit formatting where appropriate, and rerun the build. Lint diagnostics are not counted as application tests or coverage; existing OCI assertions supply the image's JUnit and its packaging-only coverage applicability remains explicit.

`.dockerignore` controls the image context, not whether native checkers can see their configuration. The planner preserves `.hadolint.yaml`, `.hadolint.yml` and `.editorconfig` in the private checking workspace even when native Docker ignore rules exclude them from `COPY`. Their bytes remain part of captured input identity. The BuildKit context retains its separately verified native file list.

## Provisioning and supported execution

Host development tasks need provisioned `hadolint` and `dockerfmt` on PATH. The CI native probe supports Windows x86_64, Linux x86_64 and macOS x86_64/arm64. The current captured BuildKit profile remains Linux amd64 and requires the existing explicitly provisioned rootless worker/AppArmor capabilities. This change does not expand captured executor platforms.

Rebuild the default toolchain image to include the new tools:

```text
docker build -f tooling/images/docker-metadata.Dockerfile -t oyzu-toolchain/docker:buildkit0.25.0 .
```

Image provisioning downloads checksum-pinned Hadolint assets and builds dockerfmt from its Go module/checksum lock using the Go 1.24 tool dependency mechanism. Each downloaded asset is limited to 128 MiB and must match its pinned SHA-256 before being installed; oversize and checksum errors are reported separately. Downloads happen during explicit provisioning, not project build execution. The image preserves upstream licenses/notices and records its immutable identity in build evidence. Native preparation checks tool availability and retains their reported versions in the Docker dependency record. The formatter's development-build version string does not replace its pinned source version and image identity.

Custom `--image docker=<image>` profiles now need both quality executables as well as BuildKit and the native metadata adapter. Older images fail preparation until rebuilt. This is an intentional change from the previous profile, which advertised lint/format checks as unavailable. A native lint or formatting violation can now make an otherwise buildable image fail its build gates.

For explicit local test provisioning, use `python tooling/provision-hadolint.py --destination <tools-directory>` and build the tracked dockerfmt tool from `tooling/images/docker-quality` with a provisioned Go 1.24.1+ compiler. The [CI workflow](../../.github/workflows/build.yml) shows the exact native download, verification and build commands. These repository scripts are not a tool-installation feature of the Oyzu CLI.

## Verification and limits

```text
python tooling/test-docker-quality.py --cli <compiled-oyzu> --tools <provisioned-tools-directory>
```

The native probe tests task listing without tools, actual lint failures and native suppression, malformed configuration, read-only format failures, explicit writes, EditorConfig overrides, missing tools and task replacement. Windows native checks passed; cross-host CI confirmation is tracked in [implementation status](../implementation-status.md).

The compiled-CLI Linux suite additionally requires successful quality gates for the snapshot image, each quality failure blocking artifacts, unchanged source and checker configuration remaining available while absent from image layers. Those assertions require isolated CI execution; native host tests do not prove them.

Hadolint is a static linter, not a container vulnerability scanner or proof of policy compliance. Native suppressions can change its result. Dockerfmt has documented syntax limitations, including escape directives and some shell grouping forms; support for every Dockerfile dialect is not claimed. Native configuration/tool behavior is described by [Hadolint](https://github.com/hadolint/hadolint#configure) and [dockerfmt](https://github.com/reteps/dockerfmt#configuration). Unsupported forms and broader scanner integrations remain part of the build-system work.

Literal bases can now use [captured provisioned images](docker-images.md). Dockerfile-free packaging, registry image/dependency acquisition, platform matrices and Helm image bindings remain unfinished. Quality checks do not make those scenarios implemented.

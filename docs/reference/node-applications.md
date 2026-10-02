# Node frontend directory artifacts

The experimental `node/app` builder recognizes a conventional Vite build script and declares its native output as a versioned directory artifact. No output section or new Oyzu setting is required. This increment supports standalone package roots; npm workspace application outputs, executable Vite configurations, arbitrary custom-script output inference and platform matrices remain unfinished.

## Minimal project and native customization

A package with a captured `vite` dependency and `"scripts": {"build": "vite build"}` uses Vite's native `dist` output. Ordinary package discovery selects `node/app`, so a standalone project needs no `build.yaml`. The [authored frontend/image variant](../../examples/builds/materialize-directory/variants/vite/) demonstrates a grouped consumer:

```yaml
frontend:
  uses: node/app
  path: frontend
image:
  uses: docker/image
  path: image
  materialize:
    - from: frontend
      to: site
```

The image's Dockerfile can use `COPY site/ /site/`. It receives `site/index.html` and the generated assets, with no extra `dist` layer. The consumer uses the producer's recorded artifact identity and matching execution platform.

The executor assembles the private BuildKit context from selected source files plus explicit materializations. Directory inputs retain all artifact descendants, including empty directories and names such as `dist` that source capture normally excludes. It rejects linked inputs, collisions and paths outside the image target; combined inputs are bounded to 100,000 selected entries and 10 GiB of file bytes. Directory copying uses the shared snapshot traversal and verifies that its content identity did not change during the copy. Native source ignore rules do not filter explicit artifact inputs.

For a different directory, use the native script `vite build --outDir public-site`. The currently recognized grammar is exactly `vite build` with an optional literal `--outDir <path>`. Paths must be contained, portable and use ASCII letters/digits, slash, underscore, hyphen or dot; parent/dot segments, absolute paths, shell expansion, quoting and compound commands are not interpreted. Vite defines the [native default and outDir option](https://vite.dev/config/build-options.html#build-outdir); Oyzu does not assume that every Node build script follows that convention.

Static discovery records the output profile and configuration-file evidence without executing project code. A Vite config file or unsupported flags fail captured planning with an explicit diagnostic, because the native output location cannot yet be established. Resolve that unsupported case before retrying; do not remove required project configuration merely to pass planning. Unknown custom scripts retain current native package behavior, and explicit `node/package` retains a package archive even with a Vite script. These are compatibility limits, not complete application packaging support.

## Tasks, reports and output

`oyzu run list` exposes native `build` and `test` scripts, inferred lint/read-only format checks and explicit mutating formatting. `oyzu build` retains the existing dependency preparation, native build, test, lint and format-check gates before packaging. For recognized Vite projects, fallback ESLint includes browser globals alongside Node globals. The selected native output directory is excluded from implicit quality traversal; supplied native scripts/configurations remain authoritative. See [Node quality](node-quality.md).

Native Node tests produce JUnit and LCOV through the existing reporting adapter. The versioned primary artifact appears beneath `dist/<target>/artifacts/application-<snapshot-version>` in the build bundle; consult its manifest `path` rather than constructing this path in consumers. Only the selected native output tree is staged. Source, dependencies and tests are not automatically copied into that artifact. Failed tests or quality checks block artifact publication to the bundle; collected failure reports remain available.

The manifest records `kind: directory`, version, producing action, complete inventory and logical tree digest. Collection and `oyzu inspect dist` use the shared [directory integrity contract](directory-artifacts.md). Missing output, links, special file types and oversized staging fail packaging. A failed operation can leave partial unrecorded output; it is not a valid artifact. `materialize` copies verified content into a fresh consumer context and retains producer evidence references. This does not prove platform independence or production release eligibility.

## Prerequisites and verification limits

Tool installation is not implemented. Development tasks require a compatible provisioned Node/npm, installed native project dependencies and [quality tooling](node-quality.md). The authored variant pins Vite 8.3.2 and npm 11.11.0, with Node >=22.12. Captured acceptance uses the provisioned Node 22/npm image, the quality image, Docker metadata tooling and the isolated BuildKit worker described in [builder verification](build-verification.md). CI provisions those tools explicitly before invoking the already compiled CLI.

Dependencies are acquired through the existing npm broker/captured-cache flow before isolated execution. Build actions replay prepared dependencies offline; this output integration introduces no new download route or credential channel. Native project build scripts remain executable project code and receive the existing sandbox restrictions. Custom Vite configuration, SSR/library modes, compound build commands, native workspace application output and cross-platform directory reuse need further integration. Windows transfer of Unix executable directory metadata retains the shared directory limitation.

Rust checks cover discovery, literal output arguments, package/application distinction and unsupported-config diagnostics. `tooling/test-node-application.py --cli <compiled-path>` executes native Vite development tasks and staging, including browser lint, unchanged source and custom output. CI runs that probe on Windows, macOS and Linux. The Docker acceptance suite builds the authored variant, verifies actual image bytes against the directory manifest, JUnit/coverage, repeatability, custom outDir, tampering and failed-producer rejection. See [implementation status](../implementation-status.md) for checks actually completed; test registration alone does not establish acceptance. EX-050's original custom script and two-platform matrix remain pending.

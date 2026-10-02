# Node application directory artifacts

The experimental `node/app` builder declares a versioned directory artifact for a recognized Vite build or an explicitly selected application with a custom native build script. No output section or new Oyzu setting is required. This supports standalone package roots, including native Vite 8 configurations and the conventional `dist/` contract below; native workspace application outputs and automatic custom-script output inference remain unfinished. Platform matrices can select matching provisioned Node images through [toolchain selection](toolchain-platforms.md); the new ARM native execution result is pending CI.

## Custom build scripts and the dist convention

For a project whose native script writes `dist/`, application intent is sufficient:

```yaml
frontend:
  uses: node/app
  path: frontend
```

The [authored directory example](../../examples/builds/materialize-directory/project/) uses `"build": "node build.mjs"`. Oyzu runs the native script through its selected manager, then stages the contents of `frontend/dist/` as the primary directory artifact `application-<snapshot-version>`. A consumer using `materialize: [{from: frontend, to: site}]` receives `site/index.html`, without a nested `dist` directory. This is a documented convention selected by explicit application intent, not static interpretation of arbitrary JavaScript. Custom scripts retain native lifecycle semantics and can use compound commands supported by their manager.

The output must be a freshly produced contained directory. Missing output, a file in place of the directory, linked entries or special files fail packaging; previous bundle contents cannot satisfy the new build. Build/test/lint/read-only format gates still apply, and the default quality checks exclude the generated `dist`. Browser globals are not inferred merely from the application selection. Static content alone does not prove platform independence: the current artifact retains its execution platform, and multi-platform reuse remains unimplemented.

Discovery exposes `builder_selection: explicit` and the `dist-application` output profile. A recognized framework such as Vite has stronger output evidence and keeps its native adapter and validation. Explicit `node/package` retains native archive packaging. For compatibility, an automatically inferred Node target with an unknown custom script still uses native package output; add `uses: node/app` only when the script follows this directory convention. A custom non-dist output requires further native output integration; no new general-purpose output configuration is introduced here. Node workspaces continue to use native workspace package planning rather than this single-root application profile.

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

Static discovery records the output profile and configuration-file evidence without executing project code. Unsupported Vite flags still fail captured planning with an explicit diagnostic. Unknown custom scripts follow the intent/convention rules above, and explicit `node/package` retains a package archive even with a Vite script. These are compatibility limits, not complete application packaging support.

## Native Vite configuration

Existing `vite.config.js`, `.ts`, `.mjs`, `.mts`, `.cjs` and `.cts` files use Vite's own loader. For example, the [configuration overlay](../../examples/builds/materialize-directory/variants/vite-config/) selects `web/production` from the native mode and emits an extra asset through a Vite plugin. Neither discovery nor dependency acquisition evaluates that configuration.

After dependency installation in the private offline workspace, preparation replaces the private package's build script with an Oyzu wrapper. The native manager still invokes its prebuild/build/postbuild lifecycle. The wrapper calls Vite 8's native builder API and observes its resolved configuration and written output. The original checkout is unchanged. A literal `--outDir` retains native precedence over the configuration's output directory. Package-manager behavior still determines which lifecycle hooks run.

The plan names `application-<snapshot-version>` before execution. Native configuration determines an intermediate output location during execution; after a successful build, packaging copies that directory into the planned artifact. The second artifact, `build-metadata`, is `vite-output-<snapshot-version>.json` with media type `application/vnd.oyzu.vite-output.v1+json`. Its bounded record includes schema version, kind, Vite version, snapshot version, native root, output directory and mode. Both artifacts participate in bundle integrity checks. Consumers that omit an artifact name still select `primary`.

The current adapter requires one finite client environment and directory output. Native root/output must remain within the captured target without linked parents. Library/SSR builds, multiple environments, watch mode and `write: false` fail explicitly. Bundler output options must agree with the resolved directory; the adapter checks before writing and again when observing written output. Native plugins remain executable project code subject to the sandbox, not trusted release evidence. A failed build does not produce a successful output record. Missing, malformed or version-mismatched metadata fails packaging; correct the native configuration or failed operation and rerun the build. Vite versions outside major 8 require another qualified API adapter.

## Tasks, reports and output

`oyzu run list` exposes native `build` and `test` scripts, inferred lint/read-only format checks and explicit mutating formatting. `oyzu build` retains the existing dependency preparation, native build, test, lint and format-check gates before packaging. For recognized Vite projects, fallback ESLint includes browser globals alongside Node globals. The selected native output directory is excluded from implicit quality traversal; supplied native scripts/configurations remain authoritative. After a captured build, implicit quality tasks use its recorded output. Before that record exists, explicit quality execution resolves native configuration in production build mode; this can execute configuration code on the host for development tasks, or inside the sandbox for captured tasks. Listing tasks remains non-executing. See [Node quality](node-quality.md).

Native Node tests produce JUnit and LCOV through the existing reporting adapter. The versioned primary artifact appears beneath `dist/<target>/artifacts/application-<snapshot-version>` in the build bundle; consult its manifest `path` rather than constructing this path in consumers. Only the selected native output tree is staged. Source, dependencies and tests are not automatically copied into that artifact. Failed tests or quality checks block artifact publication to the bundle; collected failure reports remain available.

The manifest records `kind: directory`, version, producing action, complete inventory and logical tree digest. Collection and `oyzu inspect dist` use the shared [directory integrity contract](directory-artifacts.md). Missing output, links, special file types and oversized staging fail packaging. A failed operation can leave partial unrecorded output; it is not a valid artifact. `materialize` copies verified content into a fresh consumer context and retains producer evidence references. This does not prove platform independence or production release eligibility.

## Prerequisites and verification limits

Tool installation is not implemented. Development tasks require a compatible provisioned Node/npm, installed native project dependencies and [quality tooling](node-quality.md). The authored variant pins Vite 8.3.2 and npm 11.11.0, with Node >=22.12. Captured acceptance uses the provisioned Node 22/npm image, the quality image, Docker metadata tooling and the isolated BuildKit worker described in [builder verification](build-verification.md). CI provisions those tools explicitly before invoking the already compiled CLI.

Dependencies are acquired through the selected native manager's existing broker/capture flow before isolated execution. Build actions replay prepared dependencies offline; this output integration introduces no new download route or credential channel. Native project build scripts remain executable project code and receive the existing sandbox restrictions. Vite SSR/library modes and compound Vite-command recognition, native workspace application output and cross-platform directory reuse need further integration. Windows transfer of Unix executable directory metadata retains the shared directory limitation.

`python tooling/test-node-conventional-application.py --cli <compiled-oyzu-path>` exercises the custom script's actual native build/test/quality tasks and directory staging on the host, including missing/wrong-kind output failures. The captured Node suite also builds the authored `frontend` target, then a single-platform consumer variant, checks versioned directory and OCI contents, repeatability and missing-output failure. These are separate scopes: host staging does not prove isolated execution, and the single-platform variant does not establish full EX-050 matrix acceptance. See [implementation status](../implementation-status.md) for revision-specific results.

Rust checks cover non-executing discovery, literal output arguments, package/application distinction and configured output declarations. `tooling/test-node-application.py --cli <compiled-path>` executes native Vite development tasks and staging, including browser lint, native configuration/plugin assets, npm lifecycle hooks, output overrides, quality exclusions and rejected failures. CI runs that probe on Windows, macOS and Linux. The Docker acceptance suite builds the plain and configured variants, verifies actual image bytes against the directory manifest, JUnit/coverage, repeatability, custom outDir, metadata, tampering and failed-producer rejection. The configured native probe uses Vite 8.3.2 with an ESM config; every supported native loader form and package manager still needs equivalent qualification. See [implementation status](../implementation-status.md) for checks actually completed; test registration alone does not establish acceptance. EX-050's original custom script and two-platform matrix remain pending.

The Docker suite additionally runs the original EX-050 two-platform configuration with matching provisioned amd64/ARM Node images. It requires independently executed tests/coverage and quality, two directory snapshots, exact Docker copies, a complete OCI index and repeatability. This new result remains pending; it does not establish platform-independent directory reuse or complete every authored negative case.

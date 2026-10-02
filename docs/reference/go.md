# Go applications, libraries and workspaces

The experimental Go builder discovers `go.mod` and `go.work` without Oyzu configuration. A workspace owns its declared modules. Static task listing does not run Go; explicit development execution and captured preparation use native metadata to select package patterns. Native preparation determines whether the project has main packages. A project with no main package now produces module artifacts instead of failing as an unsupported library. An explicit `uses: go/library` selects module packaging even when commands are present.

## Usage and toolchain

```text
oyzu run list
oyzu run test
oyzu run lint
oyzu run format-check
oyzu build --plan
oyzu build
oyzu inspect dist
```

Development tasks use provisioned `go` and `gofmt`. The implicit tasks include dependency installation, compile, test, vet, formatting and formatting checks. `oyzu run format` modifies source; builds use `format-check`, whose native `gofmt -l` output must be empty. Native go.work members supply the package patterns for development build/test/vet commands. Direct tests produce the test-only evidence described below; other development tasks do not produce a collected build bundle.

Captured builds require Docker and an explicitly provisioned Linux toolchain. The default CI profile is amd64:

```text
docker build -f tooling/images/go.Dockerfile -t oyzu-toolchain/go:1.24-mod0.25.0 .
```

The image extends `golang:1.24-bookworm` with the owned module packaging adapter linked against checksum-locked `golang.org/x/mod` 0.25.0. Dependency downloads occur in this provisioning step, not in project build execution. The image retains the upstream BSD license notice. Native local checks use Go 1.24.13 on Windows; new cross-host module packaging checks and isolated library builds are wired into CI but still require revision-specific confirmation.

This changes the default Go image from the base Go distribution. Custom `--image go=<image>` profiles must now include `oyzu-go-modulezip` as well as the existing compiler/native tools. Oyzu never downloads a missing image or adapter. Image content identity enters the build evidence. A missing executable fails preparation; it does not disable library packaging silently.

## Direct test evidence

`oyzu run test` instruments the implicit `go test ./...` command with `-json`, a fresh `-coverprofile` destination and `-count=1`. Native Go owns test discovery and compilation. Oyzu converts its JSON events to JUnit and retains the native coverage profile under `dist/`, with digests and summaries in the manifest. Statement coverage does not claim branch or line coverage. Packages with no tests retain an honest zero-test summary; native Go may succeed in that case. Missing/malformed reports and configured coverage thresholds can still fail the invocation.

For a `go.work` root, native workspace metadata selects contained member package patterns and the invocation combines their actual test events and coverage. Members outside the task root fail. Forward selectors through `oyzu run test -- -run TestGreeting`; forwarded arguments go only to the requested task. The default disables reuse of Go's cached test results, while Go may still reuse its native compilation cache.

The shared collector normalizes JUnit before `post_test`, exposing `OYZU_TEST_REPORT` and `OYZU_COVERAGE_REPORT` to hooks. Final validation and capture follow the hook, so report transformations are retained. Invalid native event streams or preexisting normalized outputs remain failures even if a hook subsequently writes valid XML. A failed test skips its success-only post hook and retains available evidence.

Exact `argv = ["go", "test", "./..."]` and `run = "go test ./..."` overrides receive the same instrumentation. Other custom bodies remain unchanged and must write their required JUnit and Go coverage reports. An explicit coverage declaration can redirect the native profile; an explicit JUnit declaration selects file-based reporting and requires the custom task or hook to produce that XML. The default Go command itself emits JSON, not JUnit. Declared paths must be fresh; see [direct report binding](direct-tests.md).

This workflow needs a provisioned Go compiler and available dependencies, without Docker or a platform account. Host execution retains native environment/network behavior; use existing dependencies and native `GOPROXY=off`, `GOSUMDB=off`, `GOTOOLCHAIN=local` settings when verifying an offline fixture. It is not an isolated build, does not produce application artifacts and confers no publishing authority. [Direct test evidence](direct-tests.md) describes host provenance, output preservation, errors and recovery.

`python tooling/test-go-direct.py --cli <compiled-oyzu-path> --go <native-go-path>` exercises real modules and workspaces, report inspection, test selection, failures, skips, empty suites, hooks, thresholds and custom-command obligations. CI runs it with provisioned Go 1.24.13 after compiling the CLI on each host. Current local and cross-host results are recorded separately in [implementation status](../implementation-status.md).

## Preparation and execution

Native Go commands own manifest parsing, workspace membership, main package discovery, dependency selection and cgo detection. Captured inputs must contain local modules and replacement paths; escapes and missing inputs fail. Public module acquisition uses the existing scoped Go proxy broker and go.sum verification. Private source routing and direct VCS acquisition remain unfinished. The compiler/ABI is recorded for cgo, and execution cannot turn off required cgo support through an environment override.

Preparation captures the native dependency cache and module artifact candidates. Library candidates come from captured source before build outputs or private compiler caches exist. The build then runs compile, native tests, vet and read-only formatting checks with denied networking and the prepared module cache. Final artifact collection occurs only after required gates pass. Prepared candidates are not successful build artifacts by themselves.

Application builds retain their existing single primary binary or named multiple binaries, using `-trimpath` and disabled ambient VCS stamping. Libraries compile package patterns without fabricating an executable. Native test JSON becomes JUnit, and native Go coverage retains its statement denominator. Shared collection stores these under `dist/`, with paths, digests and summaries in `dist/manifest.json`. Failed tests, vet, formatting, missing reports or compilation prevent final artifact collection. Source files remain unchanged during captured execution; inspect task logs and reports to repair the failing native input.

## Native module artifacts

Each library/workspace module produces three independently identified files:

| Artifact | Contents |
| --- | --- |
| `module-<index>-<version>.zip` | Native Go module archive with the `<module>@<version>/` prefix |
| `module-<index>-<version>.mod` | The exact projected go.mod included in that archive |
| `module-<index>-<version>.info` | Native proxy version metadata; no invented release timestamp |

Indices follow the captured, sorted module directory order. Manifest artifact names are `module-<index>-zip`, `module-<index>-mod` and `module-<index>-info`. Each artifact records its native module snapshot version. `dist/dependencies/<target>.json` records module paths/directories, filenames, versions and native `h1:` zip/go.mod checksums in `oyzu.dev/go-module-artifacts`, allowing later publication code to map files to registry identities. Publishing itself is not implemented by this increment.

Versions use the build's source-derived `-dev.g...` suffix. Native module path rules enforce the major version: a `/v2` module receives a `v2.0.0-dev.g...` snapshot rather than an invalid `v0` version. These are snapshot prereleases, not Go VCS pseudo-versions or a claim that a release tag exists. Current source-derived version policy is experimental; trusted release classification remains separate work.

The adapter uses Go's native [module zip library](https://pkg.go.dev/golang.org/x/mod@v0.25.0/zip) for allowed files, nested-module/vendor exclusions, case collisions and archive limits. It uses native module/manifest APIs for identity and projection, and native `dirhash` for checksums. It does not reimplement zip rules or infer source files from extensions.

Workspace dependencies are projected to their matching snapshot module versions. Native package imports, including test imports, supply local dependency edges even when a workspace member has no explicit require entry yet. Contained local replacements to a captured workspace module with the same module identity are removed from published go.mod files. The source manifests remain unchanged. These files use the Go [module proxy protocol](https://go.dev/ref/mod#module-proxy); independent consumer checks provision them into a local file proxy and compile a separate project with a fresh module cache.

## Verification and limits

The checked-in [library variant](../../examples/builds/go-app/variants/library/) has no Oyzu configuration. The native probe is:

```text
python tooling/test-go-modulezip.py --adapter <provisioned-modulezip> --go <provisioned-go>
python tooling/test-go-metadata.py --go <provisioned-go> --cli <compiled-oyzu>
```

The module probe checks single-module and workspace consumption, major-version identity, matching go.mod bytes, native checksums, repeatable bytes, unchanged inputs, projected implicit dependencies and invalid local replacement rejection. Rust checks cover inferred/explicit library planning and artifact path admission. CI additionally requires measured test/coverage evidence and snapshot artifacts from an actual captured library build, repeats it for byte identity, then verifies a real failed test blocks collection.

Registry replacements that require different publication semantics, local replacement modules outside the selected workspace, private/VCS routing, cross compilation, module publication and standalone task report bundles remain unfinished. Unsupported replacement projection fails preparation rather than producing a module known to depend on an unreproducible local path. Native source archives do not claim a compiled ABI or production provenance. Full cgo/sysroot portability and all authored scenario acceptance remain part of the active goal; see [implementation status](../implementation-status.md).

Platform matrices can select matching provisioned Go toolchains through [platform selection](toolchain-platforms.md). Preparation verifies actual Go OS/architecture, and tests execute in each selected runtime. The authored Go-to-container matrix is now required by Linux CI; its new ARM execution result is pending. General cross-compilation, ABI admission and all EX-027 cases are not established by this selection capability.

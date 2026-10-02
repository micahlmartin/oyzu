# EX-020: Native npm workspace dependency graph

Status: **design contract for review**. Intended hosts: Windows, macOS, Linux. See the [validation scope](../../VERIFICATION.md).

## Purpose

- The app consumes a shared package through workspace metadata.
- Changing shared code affects the app; changing app code does not rebuild unrelated targets.

## Review the project

Open the checked-in project roots: `project`. Source, native manifests, and Oyzu configuration are included here so we can review the intended experience directly.

The intended Oyzu interface is:

```text
oyzu build
```

`oyzu run list` also exposes an implicit `build` task. After installing the project's native dependencies, `oyzu run build` invokes declared member build scripts in dependency order when no root build script exists. Members without a build script are reported as having no requested compilation. This development task does not package snapshot artifacts; `oyzu build` owns the captured build and dist bundle.

Local `lint` and `format-check` also compose native member scripts with inferred defaults; a scripted member does not suppress an unscripted sibling's checks. Defaults inspect root-owned source and exclude member/nested-member directories owned by other checks. Explicit root scripts take precedence. `oyzu run format` permits formatting writes, while captured builds request read-only formatting validation. Native quality tools must already be provisioned.

No build YAML is required for this scenario. Configuration, where present, demonstrates only the feature under discussion.

## Native checks available now

With the named toolchains already installed, run:

```text
# From project
npm install --offline --ignore-scripts --no-audit --no-fund

# From project
npm test --workspaces
```

Native commands document the underlying ecosystem workflow. They are supporting context, not a prerequisite for accepting or revising this design contract.

## Failure and variation cases

- **publishable-root:** Remove `private: true` from the root package and add conventional root source and tests. Expected: a third snapshot package, separate root JUnit/coverage, no duplicate member tests, no engine state in archives, and a failed root test blocking artifacts. No Oyzu configuration or root test script is needed. The native probe and compiled-CLI scenario suite construct this variation from the checked-in project.
- **duplicate-execution:** Select both workspace root and package targets. Expected: Do not run the same native workspace task twice.

## Contract and limitations

Acceptance criteria: BUILDER-02, PLAN-05. See the [design catalog](../../../docs/examples.md) and [scenario input](scenario.json).


Files such as `scenario.json` and sibling expectation data belong to the example harness, not to Oyzu project configuration. They define observable targets without inventing an implementation or a policy programming language.

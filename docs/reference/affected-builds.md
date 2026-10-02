# Affected target builds

`oyzu build --affected <git-ref>` is experimental target-level selection for a local Git baseline. It compares the captured source bytes with that commit, selects changed targets and their declared consumers, then includes the prerequisites needed to build them. It does not reuse cached actions or artifacts.

```text
oyzu build --affected HEAD --plan
oyzu build --affected origin/main
oyzu inspect dist
```

The reference must already exist locally. Oyzu does not fetch it. Git must be available on PATH; selected builds require the same explicitly provisioned images and dependencies as ordinary [builds](build-tasks.md). `--affected` cannot be combined with positional target selection. Omitting it preserves the existing full-build behavior. No new project configuration is required.

## Selection and evidence

Comparison uses raw committed Git blobs and the immutable source snapshot, including deletions and executable-bit changes. Projects nested inside a Git repository are compared with their corresponding subtree. The same source exclusions apply to both sides, so generated `dist`, `.oyzu`, dependency and other excluded directories do not trigger builds. Checkout filters and line-ending conversion can make working bytes differ from committed bytes; such differences count as changes.

A changed file selects every enclosing discovered target. The selection follows consumers of `depends_on`, `materialize.from` and resolved cross-target task prerequisites transitively. Existing selection then adds required producers and admitted task/hook dependencies. Every selected target receives its normal applicable checks and packaging stages. A native workspace remains one target; this does not select individual native members or infer arbitrary cross-project file reads. Declare inter-target dependencies for those relationships.

The frozen plan and manifest record `extensions["oyzu.dev/selection"]` with `mode: "affected"`, requested impacted targets, the final selected set and excluded targets. Its `affected` object records the reference, resolved baseline commit, source digest, changed paths, per-target reasons and any full-build fallback. These are selection facts, not release eligibility or trusted CI evidence.

If nothing changed, `--plan` returns an empty plan without replacing `dist`. A build writes a successful, inspectable empty bundle with no tools, actions or artifacts; previous bundles retain normal history behavior. Excluded targets are not represented as successful builds, and old artifacts are not copied into the new bundle.

## Conservative fallback and limits

Oyzu selects the full workspace when the local baseline cannot be read, when an input is new relative to it, when a changed input has no target owner, or when recognized build/configuration/lock metadata changes. Root tasks with cross-target prerequisites also force full selection when inputs change. Managed configurations currently force full selection until policy-aware narrowing is implemented. The recorded fallback explains these decisions; fetch a missing baseline yourself and rerun if appropriate.

Selection does not grant network access, bypass validation, change source files or establish authorization. Source discovery and validation still cover the whole workspace. Baseline reads need no network, but normal selected-target preparation can require approved acquisition routes. Git object limits or unsupported baseline entries also cause full selection. Fallback can therefore require toolchains even when a no-change build would not.

Changes to external configuration, tools or environment are not inferred from Git. Use a full build when those inputs change. Action-level caching, native member-level impact, automatic CI baseline discovery and policy-controlled narrowing remain unfinished. Git metadata never grants signing or production publication authority.

## Verification

The compiled Windows CLI passed no-change plan/build/inspection for both repository-root and nested project roots, plus rejection of conflicting positional selection. Rust tests cover owners, transitive consumers, deletion, immutable capture and conservative fallbacks. `tooling/test-affected-selection.py` runs after CLI compilation in the three-host task matrix; cross-host results remain pending.

The Linux Node suite registers `affected-targets` against the [EX-033 declared-target variation](../../examples/builds/affected-graph/README.md): no-change output, API-only selection with its producer, shared-input consumers, snapshot packages and required reports. Its captured-build result remains pending. This variation does not complete EX-033's original intra-project action-cache contract.

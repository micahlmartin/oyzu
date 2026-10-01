# Build configuration implementation contract

Status: proposed v1alpha1. This document selects initial semantics so implementation does not need to invent them. Existing agreed minimal syntax is preserved.

## Boundaries and precedence

Read protected management indicators first. Establish the nearest Git worktree root; outside Git, use an explicit `--root`, otherwise the invocation directory. Never walk above that boundary. A symlinked invocation directory is resolved before boundary selection. Root `build.yaml` defines the build inventory; nested `oyzu.toml` affects only targets inside its subtree. Nested build.yaml is an error with the supported root path, not silently merged configuration. Repository submodules are separate source imports and are not automatically traversed.

User defaults live in the OS application configuration directory under `oyzu/config.toml`. A project cannot redirect that directory or management records. Resolution is builtin defaults → managed defaults → user defaults → root-to-target TOML → eligible local overrides → CLI requests. Mandatory managed constraints validate the result independently. Managed defaults intended to be immutable must be expressed as constraints. Global tools never satisfy a locked build requirement merely by appearing first on PATH.

Tables merge recursively; arrays/scalars replace; a task replaces as one unit. Unknown keys, duplicate keys and conflicting scalar/table types fail at their source location. YAML uses the JSON-compatible scalar subset: reject custom tags, merge keys and aliases. Limit each config file to 1 MiB, nesting to 32 and total effective target count to 1,024 before expansion. Paths use repository-relative forward-slash spelling; `.` is the root. Reject absolute paths, parent escapes, Windows drive prefixes, reserved device names and portable case collisions before selecting an executor.

`oyzu.local.toml` participates in ordinary local development and local builds; its nonsecret semantic values and source digest are captured. All detected CI contexts ignore it by default, even if workload identity is not verified. A caller can request local settings but managed CI must expressly permit that request. No local setting can weaken enforced policy. A bare `CI=true` can restrict behavior but cannot grant privilege.

## Target configuration

The root YAML is a map from target name to target object, without a mandatory version/header. Target names match `[A-Za-z][A-Za-z0-9_-]{0,63}`; reject portable case collisions. Core fields are `uses`, `path`, `depends_on`, `materialize`, `platform`, `matrix`, `container` and `bindings`. `uses` is required for an explicit target. Builder-specific fields must be registered, versioned and validated; there is no freeform bag of silently ignored properties. The draft [build schema](../../contracts/v1alpha1/build.schema.json) defines shapes; semantic checks enforce meaning.

`path` defaults to `.`. Multiple explicit targets may share a source path when their builders differ; two targets claiming the same native build ownership unit are rejected. When root YAML exists its explicit targets are the selected inventory; native submodules and dependencies are still discovered under each owner. Do not also add a competing zero-config root target. No YAML means discover conventional project roots, excluding VCS internals, dependency/cache/output directories and vendor trees identified by the native manager.

`depends_on` adds ordering only. It never mounts another target's files. `materialize` selects captured outputs and creates data dependencies. `bindings` selects typed artifact metadata, initially only Helm image digest binding described in OEP-0018. Both are finite records, not expressions. There are no includes, template evaluation, conditionals or user-defined operators.

`platform` and `matrix.platform` are mutually exclusive. Additional matrix keys initially are python, node, go, rust and java, each an array of exact locked tool versions, with at most one language axis appropriate to the builder. Duplicate values fail. Cartesian expansion has at most 64 variants per target and 4,096 actions per plan by default; explicit invocation limits may raise these up to administrative caps. Enforce before acquisition. Unknown axes fail. List ordering does not influence variant identity.

## Tools, tasks and environment

Existing `[tools]`, `[env]` and `[tasks]` tables remain. Build-related cache fields are `cache.remote` (OCI URI), `cache.read` (boolean, default true) and `cache.write` (boolean, default false for a remote). Local action caching defaults on. Managed policy supplies equivalent effective settings. No cache-key, restore-path or save-step configuration is exposed.

Environment values are literal strings. No shell evaluation occurs on read. Builder-required nonsecret environment is merged explicitly; reserved authentication/management variables cannot be replaced by project configuration. Ambient environment is excluded except a documented executor bootstrap allowlist (OS process necessities), which is separated from the action environment. Secrets belong to broker handles, never `[env]` for builds. Do not dump environment values in diagnostics; origin inspection redacts sensitive settings.

Task fields and platform-specific command behavior are fully specified in [OEP-0005 implementation](../OEP-0005-tasks-and-hooks/implementation.md). A build override's inputs, dependencies and report requirements are validated before execution. Do not interpret a TOML task called preflight as permission to replace engine guards.

## Validation, evolution and verification

Validate syntax, resolve origins, validate native ownership, then enforce policy. Errors identify file/line, field, target and a remedy without disclosing tokens. `oyzu config explain --json` emits value origins and redacted restrictions; it is not a credential export API. CI receives deterministic noninteractive diagnostics instead of prompts.

The implicit file dialect is initially v1alpha1 tied to the CLI compatibility range. Unknown fields fail, so newer configuration cannot silently behave differently on older clients. Add an explicit format-version field only when a real incompatible migration exists; migrations require a reviewable diff, never automatic source rewriting during build.

Verification extends CFG-01 through CFG-06 with worktree roots, nested TOML, multiple explicit targets sharing a path, YAML aliases, path/case collisions, bounded matrices, CI-local overrides and configuration origin snapshots. License choice and OS management-store ACL implementation remain external prerequisites; these parsing semantics are ready to implement as draft contracts.

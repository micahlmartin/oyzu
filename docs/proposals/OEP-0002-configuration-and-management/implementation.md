# Configuration resolution implementation contract

Status: proposed v1alpha1; companion to [OEP-0002](README.md).

## Ownership and typed interfaces

The shared configuration component separates `sources`, `registry`, `profiles`, `merge`, `constraints`, `compatibility`, `resolve`, `explain`, and `edit`. These are responsibilities, not a requirement for one crate per module. CLI, agent and optional desktop consume the same resolver. Builders register typed setting descriptors and consume resolved values; they must not independently search home directories or reinterpret policy.

Core Rust interfaces use owned, immutable records: `ConfigSource` (scope, identity, parsed syntax and digest), `SettingDefinition` (see [registry](settings.md)), `RequestedConfig`, `EffectiveConfig`, and `ResolutionReport`. `resolve(sources, registry, context, selection, constraints)` returns an effective snapshot or structured diagnostics. Origins include source, key span, scope, profile, replacement/removal history, and contributing constraint IDs. A `VerifiedPolicySnapshot` can only be constructed by the verification component. Parsing an ordinary JSON object cannot construct one. Filesystem and clock access sit behind testable adapters.

## Locations and protection

| OS | Machine directory | User configuration | User state |
| --- | --- | --- | --- |
| Windows | Native ProgramData known folder plus `Oyzu` | Native RoamingAppData plus `Oyzu/config.toml` | Native LocalAppData plus `Oyzu/state` |
| macOS | `/Library/Application Support/Oyzu` | `~/Library/Application Support/Oyzu/config.toml` | `~/Library/Application Support/Oyzu/state` |
| Linux | `/etc/oyzu` | `$XDG_CONFIG_HOME/oyzu/config.toml`, otherwise `~/.config/oyzu/config.toml` | `$XDG_STATE_HOME/oyzu`, otherwise `~/.local/state/oyzu` |

The machine directory contains optional `config.toml`, `admin-settings.json`, and `management.json`. Use native Windows known-folder APIs rather than trusting environment variables for protected paths. Linux user XDG paths must be absolute; invalid values produce a diagnostic and use the standard user fallback. Project configuration cannot relocate any of these roots.

Unix administrative files and relevant parent directories must be root-owned and not writable by ordinary users. Windows ACL validation must exclude ordinary-user write, replacement, ownership and parent deletion rights; a read-only file attribute is insufficient. Reject links/reparse redirection for administrative files and revalidate the opened handle against the checked identity. An unreadable or insecure existing protected record is an error, never equivalent to absence. Administrative provisioning and removal require OS administrative access; ordinary CLI settings cannot change enrollment. No custom device certificate is required.

## Workspace and source discovery

Resolve invocation paths before discovery. Use explicit `--root` when supplied, otherwise the nearest Git worktree root, otherwise the invocation directory. The explicit root must contain the selected targets and cannot redirect protected sources. Never ascend beyond it. Submodules are separate imports, not implicit configuration parents.

Read project files from root to each selected target. Normal files form one pass, then eligible `oyzu.local.toml` files form another root-to-target pass. Thus a root local override wins over a nested committed value. Each target receives its own scoped snapshot; nested settings cannot affect siblings. Invocation-wide settings are accepted only at root or broader scopes.

Root `build.yaml` owns explicit target inventory; nested build files are rejected when encountered for a selected target. Without YAML, discover conventional projects while excluding VCS, output, dependency, cache and native vendor directories. With YAML, do not add a competing implicit root target. Distinct builders may share a path, but two targets cannot own the same native build unit.

Relative configuration paths are relative to their declaring file, then normalized. Project execution/input paths must remain inside the permitted workspace/import boundary. Host-local cache locations are separately typed host preferences and cannot become undeclared build inputs. Portable artifact paths reject absolute paths, parent escapes, drive prefixes, reserved device names and case collisions.

## Resolution algorithm

1. Inspect protected records before reading ordinary overrides or deciding whether login is needed. Obtain a verified applicable corporate snapshot when enrolled.
2. Capture source bytes, invocation arguments, detected context and registry definitions. Check parser/resource limits and duplicate keys.
3. Validate known types and legal scopes, retaining unknown optional syntax for diagnostics and edits. Validate required capabilities for participating sources.
4. Select one profile using [profile rules](profiles.md). Exclude ineligible sources before parsing their content; record the exclusion reason.
5. In precedence order apply base removals, base values, selected-profile removals, then selected-profile values. A single overlay cannot remove and assign the same key.
6. Merge values and origin history. Collect administrative constraints independently; never replace constraints through ordinary merging.
7. Intersect applicable constraints, diagnose inconsistent policies, and validate final requested values. Reject violations; do not silently clamp or silently substitute a permitted value.
8. Validate native tool requirements, locked identities, task contracts and required reports. Resolve secret references only for an authorized consumer, not while parsing.
9. Freeze the effective configuration and nonsecret provenance for planning. The executor receives that snapshot, not paths to reread opportunistically.

Missing settings inherit. Scalars and arrays replace; empty arrays intentionally replace with empty arrays. Tables merge by registered keys. Task definitions and secret-reference objects replace atomically. Types cannot change through layering. Built-in defaults are applied before every other layer.

For deliberate removal:

```toml
[overrides]
remove = ["env.DEBUG_FLAG", "tasks.api:preview"]
```

Version one permits removal of explicitly configured environment entries and tasks only. Paths use the exact registered root plus the literal entry name; no globbing or numeric indexes. Removing an absent entry warns. The tombstone removes inherited explicit configuration, and a later layer may reintroduce it. Removing a task override exposes any builder-discovered task; it cannot delete mandatory engine checks. Environment removal produces an unset operation during shell activation. Tools and administrative constraints are not removable through this mechanism. Unknown optional removal targets warn; an author requiring new removal behavior must declare its capability.

## CI and ambient environment

Context detectors infer CI; there is no writable context override. Detected CI excludes user-level computation settings, tasks and environment, and excludes all local override files by default. User presentation preferences may participate. Protected machine/admin sources still apply. `--local-overrides` explicitly requests local files; standalone installations may honor it, while managed installations require `config.localOverridesInCi = true` from policy. It does not re-enable user-global build settings. All admitted values still face constraints and provenance capture.

A spoofed `CI=true` may cause stricter defaults but grants nothing. Required checks, publication and signing use independent authorization and evidence. Builds admit only declared environment plus a documented executor bootstrap allowlist; they do not inherit a developer's complete shell environment.

Environment strings are literal, with no shell or variable expansion. Development activation may use `{ secret = "dev/api-token" }` references through the broker; build actions cannot consume those as ordinary environment literals and must use the explicit secret-consumer contract. Missing access is an error, never an empty substituted secret. Reserved authentication/management variables cannot be assigned by project files. Reject portable case collisions in environment names. Arbitrary environment values are redacted in inspection by default. Reading a config never authorizes execution of its tasks/hooks; existing workspace trust checks still apply.

## Compatibility

```toml
[compatibility]
schema_major = 1
requires = ["config.profiles/v1"]
```

Absent metadata means major 1 with no extra requirements. Requirements union across applicable sources and the active profile; they cannot be removed by overrides. Unknown optional settings warn once per source/key and remain inert. Known invalid types, illegal scopes, duplicate keys, unknown required capabilities and unsupported major versions fail before affected execution. Suggestions may identify likely misspellings. `config validate --strict` upgrades optional warnings to errors without making ordinary builds strict by default.

Inactive profiles receive syntax/type checks, but their capability and secret-access requirements do not block unrelated execution. `config validate --all-profiles` validates their requirements explicitly. YAML's minimal target map is preserved; a root TOML capability declaration gates required new build features. Unknown optional fields may be ignored, but unsupported values for understood fields, builder IDs or matrix axes fail. Authors cannot expect an old client to infer that an unknown field is mandatory.

Initial capability IDs are `config.cascade/v1`, `config.profiles/v1`, `config.removal/v1`, and `policy.constraints/v1`. The resolver reports its supported IDs. Signed enforcement records reject unknown operators or mandatory semantics even though ordinary configuration tolerates optional extensions. Strict artifact manifest and plan schemas remain strict.

Limits: 1 MiB per ordinary file, 128 participating files, 8 MiB aggregate source data, depth 32, 128 profile names, 10,000 setting entries and 1,024 targets before expansion. JSON rejects duplicate keys and nonfinite numbers. YAML rejects aliases, merge keys and custom tags. These are defensive v1 limits, not profile-overridable settings.

## Existing build and tool contracts

Explicit target names match `[A-Za-z][A-Za-z0-9_-]{0,63}` without portable case collisions. Fields include `uses`, `path`, `depends_on`, `materialize`, `platform`, `matrix`, `container`, and `bindings`; builder-owned fields register their types. `uses` is required on explicit targets and `path` defaults to `.`. Ordering edges do not mount files; materialization selects captured outputs and bindings select typed artifact metadata.

`platform` and `matrix.platform` are mutually exclusive. Initial language axes are python, node, go, rust and java, with at most one appropriate language axis, using exact locked versions. Duplicate values and unknown axes fail. Default limits remain 64 variants per target and 4,096 actions per plan; raised invocation limits remain bounded by administrative caps. List ordering does not change variant identity. Profiles cannot rewrite build inventory, builder selection, matrix topology or materialization relationships.

Global PATH entries never satisfy locked requirements by accident. A profile requesting an incompatible version fails against the existing lock and native project constraints; it does not silently rewrite the lock. Native package configuration remains authoritative for native behavior, while Oyzu adapters apply approved transport/routing and enforce effective constraints. Task replacement cannot bypass preflight, required test/coverage collection, or policy checks. See [task implementation](../OEP-0005-tasks-and-hooks/implementation.md).

## Inspection and editing

| Command | Contract |
| --- | --- |
| `oyzu config show [--json]` | Effective settings with redacted values and resolution status |
| `oyzu config get <key>` | One effective setting; redaction still applies |
| `oyzu config explain [<key>] [--json]` | Origins, selected profile, exclusions, overwritten values where safe, constraints and denial reasons |
| `oyzu config profiles` | Available names, contributing sources, current selection and reason |
| `oyzu config validate [--strict] [--all-profiles]` | Non-executing validation; nonzero on errors |
| `oyzu config status` | Enrollment, policy revision, expiry and offline availability without credentials |
| `oyzu config refresh` | Request verified refresh; no implicit login UI or authority downgrade |
| `oyzu config set <key> <value> --user/--project/--local` | Edit exactly one explicitly selected ordinary source |
| `oyzu config unset <key> --user/--project/--local` | Delete assignment in that source, revealing inherited values |

Write scopes are mutually exclusive and mandatory. Project/local default to workspace root; `--directory` selects a contained scope and `--profile` selects an overlay. Use `--json-value` for structured typed values. Reject unknown keys in `set` because the CLI cannot type them; hand editing remains possible. `unset` is distinct from an explicit removal tombstone. Ordinary editing commands cannot modify policy/bootstrap/cache.

Use a lossless syntax tree, preserve comments and unknown fields, compare the original digest before atomic replacement, and report the destination. Concurrent modifications cause `CONFIG_EDIT_CONFLICT`. Explain may report an unresolved request without executing it. Never write secrets into source, logs, plans or error suggestions. Help, status and diagnostics remain usable when execution is blocked.

## Snapshot identity and error model

Compute a domain-separated SHA-256 digest (`oyzu.config.v1` followed by a zero byte and canonical JSON) of normalized effective computation settings, effective execution settings and required behavior. Exclude credentials, display preferences, incidental host cache paths, timestamps and source formatting. Secret references are redacted in public provenance and secret bytes never enter this digest; any secret-dependent action follows the executor's non-cacheable or explicitly safe contract. Profile names and origins are provenance, while the effective values determine computation identity. Policy revision and authorization evidence remain separately recorded.

Refresh does not mutate an active plan. If a fresh authorization requires different computation/checks, stop and replan; otherwise record the new authorization separately. Never relabel earlier evidence as if new checks ran.

Diagnostics contain stable code, severity, key, source span where available, target, safe message and remedy. Initial codes: `CONFIG_SYNTAX`, `CONFIG_DUPLICATE`, `CONFIG_LIMIT`, `CONFIG_UNKNOWN_OPTIONAL`, `CONFIG_INVALID_VALUE`, `CONFIG_SCOPE`, `CONFIG_PROFILE_UNKNOWN`, `CONFIG_REQUIRED_CAPABILITY`, `CONFIG_MAJOR_VERSION`, `CONFIG_POLICY_CONFLICT`, `CONFIG_OVERRIDE_DENIED`, `CONFIG_EDIT_CONFLICT`, `POLICY_UNAVAILABLE`, `POLICY_INVALID`, `POLICY_EXPIRED`, `POLICY_CLOCK_UNCERTAIN`.

## Implementation and verification sequence

1. Introduce typed descriptors, lossless parsing and raw-source preservation around existing configuration behavior.
2. Implement bounded discovery, OS paths, deterministic merging and provenance fixtures.
3. Add profiles, removal, compatibility declarations and inspection/editing commands.
4. Add protected local JSON administration and constraint evaluation without a platform dependency.
5. Add verified corporate snapshots, atomic cache/state transitions and refresh through the headless agent.
6. Integrate effective settings with native builders, locks, shell activation and fresh privileged authorization.

Use unit tests for merge/constraint algebra, golden resolution diagnostics, parser fuzzing, filesystem boundary and ACL tests on all OSes, concurrent-edit/cache fault injection, identity/scope replay tests and end-to-end standalone/managed CLI scenarios. [Verification cases](examples.md) map expected outcomes. Documentation checks and syntax parsing alone do not establish runtime compliance.

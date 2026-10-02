# Cascading configuration (experimental)

The configuration engine implements the public OEP-0002 draft in Rust. Its CLI and wire formats remain experimental; implementation authorization does not change the proposal's maintainer-review status. See the [acceptance contract](../proposals/OEP-0002-configuration-and-management/README.md) and [measured implementation status](../implementation-status.md).

## Prerequisites and quick start

Use an Oyzu CLI built from this branch; these commands are not a promise about older releases. Standalone configuration inspection and editing require no platform account, Docker, native ecosystem toolchain or network service. The client has Windows, macOS and Linux implementations; platform qualification and measured checks are tracked in [implementation status](../implementation-status.md). Running tasks or builds additionally requires their explicitly provisioned native tools.

In an otherwise empty directory named `config-demo`, save this as `oyzu.toml`:

```toml
[build]
jobs = 2

[profile]
default = "dev"

[profiles.dev.build]
jobs = 3

[ui]
color = "never"
```

Run these commands from its parent directory. Expected values assume standalone operation with no administrative constraints, local overrides or `OYZU_PROFILE` selection:

```text
oyzu --root config-demo config get build.jobs
oyzu --root config-demo --no-profile config get build.jobs
oyzu --root config-demo config validate --strict --all-profiles
oyzu --root config-demo --profile dev config set build.jobs 4 --project
oyzu --root config-demo config get build.jobs
oyzu --root config-demo --profile dev config unset build.jobs --project
oyzu --root config-demo config get build.jobs
```

The successive `get` results are `3`, `2`, `4`, and `2`. Validation returns JSON with `valid: true`. Removing the profile assignment exposes the base value; it does not restore the old value of `3`. Set/unset return the destination path and updated key. No project task runs in this walkthrough.

## Inspection and selection

```text
oyzu --root path/to/project config show --json
oyzu --root path/to/project config explain build.jobs --json
oyzu --root path/to/project config profiles
oyzu --root path/to/project config validate --strict --all-profiles
oyzu --root path/to/project --profile integration run test
oyzu --root path/to/project --no-profile run test
```

`--root` fixes the configuration boundary. Without it, the nearest Git worktree root is used, otherwise the invocation directory. `-C` / `--directory` selects an invocation directory. When only `--root` is supplied, that directory is also the invocation directory. A directory nested in a Git checkout therefore no longer implicitly creates a separate configuration boundary; use `--root` for standalone example projects.

Relative task `cwd` values are now relative to the TOML file that declares them. For a root-file override of `api:test`, use `cwd = "api/checks"` to select that directory. Multiple targets cannot own the same native build unit; use distinct project paths or distinct builders where appropriate.

Sources resolve in this order: built-ins, verified corporate defaults, protected local administrative defaults, machine configuration, user configuration, root-to-target `oyzu.toml`, eligible root-to-target `oyzu.local.toml`, and invocation compatibility inputs. Each source applies its base followed by the selected profile. Siblings receive separate snapshots. Arrays replace, tasks replace atomically, and `[overrides].remove` removes explicit environment/task entries. Environment removals also unset inherited variables during development task execution. A task environment can replace a global variable with the same spelling, but case aliases such as `TOKEN` and `token` are rejected across the combined environment on every host.

Profile selection uses explicit `--profile` or `--no-profile`, inherited task selection, `OYZU_PROFILE`, the eligible root/default selector, inferred CI's empty `ci` profile, then no profile. Names grant no authority. Inferred CI excludes user computation settings and local files; `--local-overrides` requests local files, subject to administrative restrictions. Excluded local files are not parsed.

`config show`, `get`, `explain`, `profiles`, and `validate` do not execute project tasks. If resolution is blocked, `show` and `explain` return an explicit unresolved status without effective values; `validate` remains the failing validation command. Environment values and task bodies are redacted in configuration inspection. Discovery/task listing also redacts environment values. Explain includes source histories, byte spans, exclusions, constraint source IDs and the effective computation digest. Warnings carry severity, source byte spans when available, target scope, and a remedy without including the configured value. Unknown optional settings warn and stay inert; known invalid values, unsupported required capabilities, duplicate keys and unsupported schema majors fail. `--strict` rejects optional warnings. `--all-profiles` validates inactive capability requirements and profile outcomes.

## Editing

```text
oyzu --root path/to/project config set build.jobs 8 --project
oyzu --root path/to/project config set checks.required '["tests"]' --json-value --local
oyzu --root path/to/project --profile dev config set ui.color never --user
oyzu --root path/to/project config unset build.jobs --project
```

Exactly one of `--user`, `--project`, or `--local` is required. `--directory` can select a contained nested project scope. `unset` removes an assignment in that file and exposes inherited values; it does not create a removal tombstone. Ordinary edits remain available to repair invalid values even when resolution is blocked; they do not activate values or alter protected records. Edits preserve unrelated comments and unknown syntax, validate known types/scopes, and compare the original bytes before atomic replacement. Concurrent changes produce `CONFIG_EDIT_CONFLICT`. A small `.oyzu-config-edit.lock` coordinates Oyzu writers and is excluded from source snapshots.

Only an explicit `--profile NAME` directs an edit into that profile. An inferred/default profile does not change the write destination: an edit without `--profile` writes the base section. Scalar booleans and numbers are parsed by type; strings are literal. Use `--json-value` for arrays or objects. The array example above uses shell quoting supported by PowerShell and POSIX shells. Edits cannot repair malformed TOML syntax; correct that syntax in an editor first. They cannot modify administrative or enrollment records.

## Commands and output contracts

All `config` commands emit JSON, even without `--json`. The schema is experimental.

| Command | Result and important behavior |
| --- | --- |
| `show` | Effective values, selected profile, digest, diagnostics and resolution status |
| `get KEY` | One JSON value; fails if the key has no effective value |
| `explain [KEY]` | Resolution evidence, optionally narrowed to a key; includes origins and constraints |
| `profiles` | Available profiles, selected profile and selection reason |
| `validate [--strict] [--all-profiles]` | `valid: true` on success; nonzero exit on validation failure |
| `status` | Standalone/enrolled state and managed cache health; does not refresh over the network |
| `refresh` | Explicit online policy acquisition; returns revision and offline deadline; requires protected enrollment |
| `set` / `unset` | Changes one ordinary file and reports its destination; does not establish that the edited value is effective |

`show` and `explain` can exit successfully with `status: "unresolved"` and no effective values. `status` can likewise return an unhealthy managed state successfully. Automation must inspect these fields or use `validate` as its configuration gate. Execution errors are reported on stderr with a nonzero exit status. Inspection does not run tasks, but resolving an enrolled configuration can contact the policy endpoint and update its cache. Thus inspection is not necessarily network-free or free of persistent state changes.

## Registered settings

Ordinary settings below can appear in machine, user, project or local configuration and in profiles, subject to eligibility and administrative constraints. An absent default means no value is contributed by the registry. Recognition of a setting does not imply that its downstream integration is available.

| Key | Type and built-in default | Current behavior |
| --- | --- | --- |
| `build.jobs` | Integer 1–65535; available logical parallelism, fallback 1 | Bounds ready build actions; the smallest root/target ceiling applies |
| `checks.required` | Set of `tests`, `coverage`, `lint`, `format`; `[]` | Requires actual build check actions/report contracts |
| `checks.coverageMinimum` | Integer 0–100; `0` | Positive minimum requires coverage evidence; missing or zero-denominator coverage fails |
| `ui.color` | `auto`, `always`, `never`; `auto` | Human task labels/status only; JSON stays unstyled |
| `profile.default` | Profile name; absent | Default selector; cannot be declared inside a profile or nested project selector scope |
| `cache.remote` | OCI URL; absent | Validated transport preference; credentials, query and fragment are forbidden; remote action caching remains deferred |
| `cache.read`, `cache.write`, `cache.local` | Booleans; `true`, `false`, `true` respectively | Resolved preferences; do not enable an absent cache implementation |
| `tools.<name>` | Nonempty string; absent | Explicit requests fail execution until a verified installation binding exists; no tool installation is performed |
| `env.<name>` | Literal string or `{ secret = "reference" }`; absent | Literal environment overlay; secret references require a future authorized consumer |
| `tasks.<name>` | Task record; absent | Atomic task replacement; see syntax below |
| `docker.apparmorProfile` | Profile name; host-derived | `oyzu-buildkit` when the detected Linux user-namespace restriction requires it, otherwise `unconfined` |

Environment names use ASCII letters, digits and underscores, beginning with a letter or underscore; the `OYZU_` prefix is reserved case-insensitively. Values are not expressions or interpolation programs. Inspection redaction does not make literal environment values a secret store: literals can participate in build inputs/evidence. Do not use a literal as a substitute for the deferred secret consumer.

Task records select either `run` (a shell command) or `argv` (an argument array), and accept `shell`, `cwd`, `env`, `depends_on`, `inputs`, `outputs`, `cache`, `interactive` and `reports`. Accepted metadata is not a claim of complete caching or interactive execution support. Task `env` contains strings. Task execution also requires a discoverable native project. For example, with Node explicitly provisioned and a `package.json` containing `{"name":"config-demo","version":"1.0.0"}`, add this to `oyzu.toml`:

```toml
[tasks.hello]
argv = ["node", "-e", "console.log('hello')"]
```

`oyzu --root path/to/project run hello` executes that command. A higher-priority task record replaces the entire record rather than merging its fields. To remove inherited environment/task entries, use dotted key strings in a removal list:

```toml
[overrides]
remove = ["env.DEMO_MODE", "tasks.hello"]
```

The following keys are administrative-only and cannot appear in ordinary files or profiles:

| Key | Type and default | Meaning |
| --- | --- | --- |
| `config.localOverridesInCi` | Boolean; `false` | Whether the local override request is eligible in CI |
| `config.allowedProfiles` | Set of names, including `@none`; absent | Restricts selection when supplied |
| `tools.allowed` | Set of canonical tool IDs; absent | Restricts tool eligibility |
| `tools.catalogs` | Ordered string array; `["public"]` | Catalog preference; unavailable protected bindings block execution |
| `registries.routes` | Route array; `[]` | Each entry has `protocol`, `scope`, `connectorId`; protocols are `npm`, `pypi`, `maven`, `cargo`, `go`, `oci`, `tools`; opt-in standalone Node/Go installation supports explicit [host tool bindings](tool-management-development.md#standalone-proxy-acquisition); real Node proxy acceptance passed, Go proxy acceptance remains pending; other connector execution remains deferred |

## Files and persistent state

| Host | Machine directory | User configuration | User state directory |
| --- | --- | --- | --- |
| Windows | ProgramData known folder + `Oyzu` | RoamingAppData known folder + `Oyzu/config.toml` | LocalAppData known folder + `Oyzu/state` |
| macOS | `/Library/Application Support/Oyzu` | `~/Library/Application Support/Oyzu/config.toml` | `~/Library/Application Support/Oyzu/state` |
| Linux | `/etc/oyzu` | `$XDG_CONFIG_HOME/oyzu/config.toml`, fallback `~/.config/oyzu/config.toml` | `$XDG_STATE_HOME/oyzu`, fallback `~/.local/state/oyzu` |

Windows paths come from native known-folder APIs. Relative XDG paths warn and use the home fallback. The machine directory contains ordinary `config.toml`, protected `admin-settings.json`, and protected enrollment `management.json`. Project/local files are named `oyzu.toml` and `oyzu.local.toml`; keep personal local overrides out of version control. Oyzu does not auto-enroll the machine.

Managed state includes signed policy entries under the user state directory and integrity pointers/high-water state in the native credential store. Ordinary configuration edits do not change that state. Source capture enforces size/complexity budgets, including a 1 MiB file limit, and detects concurrent source changes rather than mixing revisions. This configuration format has no include mechanism or executable configuration language.

## Administrative policy

Native locations follow OEP-0002: Windows known folders, macOS Application Support, and Linux `/etc/oyzu` plus absolute XDG user locations. Protected files require administrative ownership and permissions. Windows uses native ACL/handle checks; Unix checks root ownership and write permissions; macOS also examines extended ACL entries on the opened object and rejects non-root mutation grants using the [native descriptor ACL API](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man3/acl_get_fd_np.3.html). An insecure, unreadable or malformed enrollment never becomes standalone mode.

`admin-settings.json` uses the same typed constraint evaluator as corporate policy. Locks, allowed whole values, numeric bounds and required sets intersect independently of ordinary precedence. Required checks cannot be removed by empty ordinary arrays. The build planner requires configured checks to have actual actions/report contracts; report collection enforces the coverage minimum, including missing/zero-denominator failures.

`config status` reports enrollment and cached-policy status. `config refresh` performs a headless request to the enrolled platform's public configuration-context endpoint. Signed responses use pinned Ed25519 keys, canonical JSON, identity/context binding, monotonic sequences, expiry and explicit offline permissions. Cache entries are content-addressed and flushed before an OS credential-store pointer commits them. A corrupt entry can only be repaired from a newly verified online response. Windows Credential Manager, macOS Keychain and Linux Secret Service hold integrity state. If that service is unavailable, managed execution fails; standalone use does not need it. Transport backoff never extends signed deadlines. Server denial cannot become offline permission.

## Existing settings migration and boundaries

The Docker adapter registers `docker.apparmorProfile`. It replaces direct adapter reads of `OYZU_BUILDKIT_APPARMOR_PROFILE`; the legacy environment variable remains a diagnosed invocation input and faces the same policy constraints. The default still detects the host's AppArmor user-namespace restriction. Builder namespaces stay behind the crate-private Builder contract.

A frozen build configuration contributes its digest to target/action records. Formatting, profile names and presentation preferences do not affect that configuration digest. Policy revision and online/offline context remain separate evidence. No policy snapshot grants signing, publishing or credential issuance.

The current native tool-store, authenticated platform-session, connector-binding, shell-secret-consumer and remote action-cache implementations are not completed by this configuration change. Explicit tool requests that cannot be bound to a verified installation are rejected before builds. Enforced routes/catalogs requiring unavailable bindings fail before public acquisition. Managed builders that still require public dependency preparation are blocked; offline-capable local Docker and Helm preparation remains separate from those network paths. Development secret references require an authorized consumer and are not substituted into build literals. `build.jobs` bounds concurrent ready actions. The invocation uses the smallest admitted root/target ceiling; each target retains serial mutation and hook order. Targets have private workspaces and output roots. Dependency or deferred-report failure blocks consumers while independent targets can finish. `ui.color` controls human-readable task labels and outcome statuses: `always` emits ANSI color, `never` disables it, and `auto` enables it only when the relevant output stream is a terminal. Task-specific resolved preferences apply to that task; root tasks use root preferences. JSON output remains unstyled. Cache preferences do not grant cache credentials or enable a cache subsystem that is absent.

These boundaries are deliberate failures rather than claims that all OEP-0002 ecosystem and platform integration acceptance cases pass. Three-host ACL/keychain qualification, signed server interoperability, authenticated logout/account-switch flows, complete connector routing and tool-store integration remain release gates.

Development execution checks tool eligibility, explicit tool requests and protected routing/catalog requirements before any prerequisite or hook runs. Managed host tasks remain blocked until approved development execution and connector bindings exist; the host runner cannot claim to enforce managed network routing. Task-specific and native-adapter environment additions must satisfy administrative environment restrictions. In a single-target nested project, the target's final task cascade determines whether a root operation is replaced or removed, consistently with inspection and build planning.

## Recovery and authentication follow-up

| Diagnostic or symptom | Recovery |
| --- | --- |
| `CONFIG_SYNTAX`, duplicate keys | Correct the source file's syntax/duplicate declaration, then validate |
| `CONFIG_INVALID_VALUE`, `CONFIG_SCOPE` | Use the registered type and permitted scope; ordinary `set`/`unset` can repair invalid values |
| `CONFIG_PROFILE_UNKNOWN` | Inspect profile declarations and select an existing name; use `--no-profile` only if policy permits |
| Optional-setting warning / strict failure | Correct the spelling or remove inert optional settings; ordinary validation does not execute unknown syntax |
| Unsupported schema major or required capability | Use a compatible client; do not remove a requirement unless the project no longer needs it |
| `CONFIG_POLICY_CONFLICT`, `CONFIG_OVERRIDE_DENIED` | Reconcile conflicting administrative constraints or choose an allowed value; unavailable execution bindings require their implementation/provisioning |
| `CONFIG_EDIT_CONFLICT` | Re-read the file and retry the intended edit; do not overwrite another writer's changes |
| `CONFIG_LIMIT` | Reduce file size, nesting or configuration entries, including inactive profile data |
| `POLICY_INVALID` | Inspect enrollment protection and policy/cache validity; an administrator may need to repair bootstrap permissions or signing configuration |
| `POLICY_EXPIRED`, `POLICY_UNAVAILABLE` | Restore endpoint/native credential-store availability and run `config refresh`; failure never converts an enrolled machine to standalone |
| `POLICY_CLOCK_UNCERTAIN` | Correct clock problems and reconcile online; do not erase rollback-protection state to bypass the error |

Offline execution requires explicit signed permission and remains bounded by both signed expiry and maximum offline age (at most 24 hours). CI requires online policy. Transport retry/backoff preserves these deadlines; denial and malformed responses do not authorize offline fallback. Explicit refresh can fail while an older valid cache remains stored. A corrupt cache entry requires a newly verified online response; damaged integrity-store state can require administrator intervention. Refresh rechecks time after I/O, so a request that finishes after a deadline does not extend authorization.

Changing protected bootstrap pins invalidates the previous cache binding. Recovery requires an online response verified against the current pins with a strictly newer policy sequence; existing high-water state remains in force. Do not delete credential-store state as a key-rotation procedure. Cache commit ordering preserves the old committed pointer until the new entry has been flushed; full crash/reboot qualification remains a release gate.

There are no login, logout or enrollment commands in this delivery. When authentication is designed, revisit session-to-policy identity binding, account/organization switching, logout invalidation, credential lifecycle, authenticated refresh and CI identity, plus end-to-end server interoperability. Configuration policy is not a login session and does not grant publishing/signing authority. Connector credentials, shell secrets, verified tool installation and remote action caching also remain separate integrations.

## Verification evidence

The [implementation status](../implementation-status.md) records tested revisions and host coverage. The current contracts have focused coverage in [resolution and editing tests](../../tests/configuration.rs), [CLI tests](../../tests/configuration_cli.rs), [execution enforcement tests](../../tests/configuration_enforcement.rs), and [signed managed-policy tests](../../tests/managed_configuration.rs). Agent transition tests beside [the policy agent](../../src/config/agent.rs) cover key rotation, elapsed deadlines and denial/clock behavior. These tests do not establish interoperability with a production authenticated service.

Documentation structure is checked with `node tooling/check-docs.mjs`; that check does not validate product behavior. The quick-start command sequence and the provisioned-Node task example were separately exercised against the compiled CLI on Windows during this documentation update.

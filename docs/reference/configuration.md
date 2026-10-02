# Cascading configuration (experimental)

The configuration engine implements the public OEP-0002 draft in Rust. Its CLI and wire formats remain experimental; implementation authorization does not change the proposal's maintainer-review status. See the [acceptance contract](../proposals/OEP-0002-configuration-and-management/README.md) and [measured implementation status](../implementation-status.md).

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

## Administrative policy

Native locations follow OEP-0002: Windows known folders, macOS Application Support, and Linux `/etc/oyzu` plus absolute XDG user locations. Protected files require administrative ownership and permissions. Windows uses native ACL/handle checks; Unix checks root ownership and write permissions; macOS also examines extended ACL entries on the opened object and rejects non-root mutation grants using the [native descriptor ACL API](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man3/acl_get_fd_np.3.html). An insecure, unreadable or malformed enrollment never becomes standalone mode.

`admin-settings.json` uses the same typed constraint evaluator as corporate policy. Locks, allowed whole values, numeric bounds and required sets intersect independently of ordinary precedence. Required checks cannot be removed by empty ordinary arrays. The build planner requires configured checks to have actual actions/report contracts; report collection enforces the coverage minimum, including missing/zero-denominator failures.

`config status` reports enrollment and cached-policy status. `config refresh` performs a headless request to the enrolled platform's public configuration-context endpoint. Signed responses use pinned Ed25519 keys, canonical JSON, identity/context binding, monotonic sequences, expiry and explicit offline permissions. Cache entries are content-addressed and flushed before an OS credential-store pointer commits them. A corrupt entry can only be repaired from a newly verified online response. Windows Credential Manager, macOS Keychain and Linux Secret Service hold integrity state. If that service is unavailable, managed execution fails; standalone use does not need it. Transport backoff never extends signed deadlines. Server denial cannot become offline permission.

## Existing settings migration and boundaries

The Docker adapter registers `docker.apparmorProfile`. It replaces direct adapter reads of `OYZU_BUILDKIT_APPARMOR_PROFILE`; the legacy environment variable remains a diagnosed invocation input and faces the same policy constraints. The default still detects the host's AppArmor user-namespace restriction. Builder namespaces stay behind the crate-private Builder contract.

A frozen build configuration contributes its digest to target/action records. Formatting, profile names and presentation preferences do not affect that configuration digest. Policy revision and online/offline context remain separate evidence. No policy snapshot grants signing, publishing or credential issuance.

The current native tool-store, authenticated platform-session, connector-binding, shell-secret-consumer and remote action-cache implementations are not completed by this configuration change. Explicit tool requests that cannot be bound to a verified installation are rejected before builds. Enforced routes/catalogs requiring unavailable bindings fail before public acquisition. Managed builders that still require public dependency preparation are blocked; offline-capable local Go, Docker and Helm preparation remains separate from those network paths. Development secret references require an authorized consumer and are not substituted into build literals. `build.jobs` bounds concurrent ready actions. The invocation uses the smallest admitted root/target ceiling; each target retains serial mutation and hook order. Targets have private workspaces and output roots. Dependency or deferred-report failure blocks consumers while independent targets can finish. `ui.color` is validated and inspectable; current output does not emit ANSI color. Cache preferences do not grant cache credentials or enable a cache subsystem that is absent.

These boundaries are deliberate failures rather than claims that all OEP-0002 ecosystem and platform integration acceptance cases pass. Three-host ACL/keychain qualification, signed server interoperability, authenticated logout/account-switch flows, complete connector routing and tool-store integration remain release gates.

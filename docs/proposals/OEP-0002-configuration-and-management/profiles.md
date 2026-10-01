# Configuration profiles

Status: proposed v1alpha1; part of [OEP-0002](README.md).

## Shape and selection

Profiles are named overlays on ordinary configuration, not separate environments or authorization identities. They are optional. A project with no profiles behaves normally.

```toml
[profile]
default = "dev"

[tools]
node = "24"

[env]
LOG_LEVEL = "info"

[profiles.dev.env]
LOG_LEVEL = "debug"

[profiles.dev.tasks.preview]
run = "npm run dev"

[profiles.integration.env]
TEST_SUITE = "integration"

[profiles.integration.compatibility]
requires = ["config.profiles/v1"]
```

Select exactly one name for the invocation. Priority is explicit `--profile <name>` or `--no-profile`, then `OYZU_PROFILE`, then the highest-precedence eligible base `[profile].default`, then inferred CI's built-in empty `ci` profile, otherwise no profile. The two CLI switches conflict. An empty `OYZU_PROFILE` is treated as absent. Environment selection is a convenience preference and cannot grant permission or disable policy. Selection must be captured in provenance.

Names match `[A-Za-z][A-Za-z0-9_-]{0,63}` and are case-sensitive; portable case collisions are rejected. Explicit unknown names fail with the available names. The built-in `ci` profile is always known but empty: required CI behavior is owned by detection and policy, not this overlay.

`profile.default` is invocation-wide and legal only in organization/local-admin defaults, machine/user sources and workspace-root normal/local files. It is forbidden in nested target sources and inside profiles. User defaults are excluded from CI profile selection along with user computation settings. CLI `--no-profile` wins over a default but cannot bypass constraints; policy may restrict selection through `config.allowedProfiles`.

## Overlay order

At each source, apply base and then selected profile before proceeding to the next source. A user profile does not outrank a project's base merely because it is a profile. For example, user base jobs 4, user `dev` jobs 8, project base jobs 6 and project `dev` jobs 12 resolve to 12; a permitted local base of 10 resolves to 10.

The set of available profiles is the union from eligible sources for selected targets plus built-ins. A selected name may have no overlay for a particular target, in which case that target retains its base values. All selected targets share the same profile name. A task-level child invocation inherits this resolved selection unless it explicitly starts a separate invocation; it cannot silently change the parent plan's profile.

Profiles may contain registered profile-eligible `tools`, `env`, `tasks`, `cache`, `build`, `checks`, and presentation settings plus removal/capability metadata. They cannot contain nested `profiles`, selectors, management settings, constraints, includes, YAML target definitions or identity claims. No profile inheritance, stacking, conditionals, interpolation or automatically executed selectors are supported.

## CI, security, and reproducibility

Selecting `ci` locally does not make execution CI. Selecting `dev` in CI does not make execution local. `production` grants no publication rights. Detection uses observed execution facts; authority uses independently verified identity and authorization.

Administrative constraints apply to every profile. Corporate and local-admin profile entries contain defaults only; mandatory rules live outside overlays. Policy decisions may depend on verified context on the server, but cannot delegate privilege to an unverified profile label.

Tool requests must still satisfy the lock and native constraints. Profile changes never implicitly regenerate a lock. Effective semantic values participate in build/action identity; a name change with identical effective behavior does not alone invalidate computation caches. The selected name and source contributions remain visible in provenance.

## Required cases

Test no profile, inferred CI, explicit selection, explicit disable, environment selection, every precedence boundary, nested defaults rejection, unknown names, case collisions, unused-profile capabilities, per-target missing overlays, forbidden nested profiles and policy rejection. Test that spoofed CI and a `production` profile cannot gain signing or publication authority.

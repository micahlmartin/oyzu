# Typed settings and administrative constraints

Status: proposed v1alpha1; part of [OEP-0002](README.md).

## Registry contract

Every interpreted setting has a descriptor: canonical key or typed map-entry pattern; value type; default; permitted scopes; profile eligibility; merge rule; validation; sensitivity; computation/transport/presentation effect; supported constraint operators; and capability/version. The registry rejects duplicate registrations. Builders own their namespaces and validators; arbitrary downloaded code cannot register privileged settings during parsing.

Ordinary key paths use literal map keys after their typed root (`tasks.api:test` identifies the complete task). Unknown optional syntax is preserved outside the typed result. Registered extension namespaces remain subject to the same rules. Configuration parsing does not execute plugins.

## Initial surface

This table defines the initial configuration families, not a promise to expose every future engine knob. Existing domain specifications retain detailed task/tool shapes.

| Key | Type/default | Scope and effects |
| --- | --- | --- |
| `tools.<name>` | Tool request; absent until declared/discovered | User/project/local/machine/admin; profiles allowed; computation; validated against lock and native requirements |
| `env.<name>` | Literal string or development secret reference | Ordinary scopes; profiles allowed; computation for declared build literals; secret restrictions apply |
| `tasks.<name>` | Atomic task record | Ordinary scopes; profiles allowed; computation; task contract validates body |
| `cache.remote` | Optional absolute `oci://` reference | Ordinary/admin scopes; profiles allowed; transport; no URL userinfo or embedded credentials |
| `cache.read` | Boolean, true | Ordinary/admin scopes; profiles allowed; execution |
| `cache.write` | Boolean, false for remote | Ordinary/admin scopes; profiles allowed; preference never grants registry write permission |
| `cache.local` | Boolean, true | Ordinary/admin scopes; profiles allowed; execution |
| `build.jobs` | Positive integer; detected logical CPU count, minimum 1 | Ordinary/admin scopes; profiles allowed; execution; maximum 65,535 |
| `checks.required` | Set of registered check IDs, empty by default | Ordinary/admin scopes; profiles allowed; computation; checks may also be mandatory from builder contracts |
| `checks.coverageMinimum` | Integer percent, 0 through 100, default 0 | Ordinary/admin scopes; profiles allowed; computation; never implies missing required coverage is acceptable |
| `ui.color` | `auto`, `always`, `never`; default `auto` | Ordinary scopes; profiles allowed; presentation |
| `profile.default` | Optional profile name | Root/broader only; no profile nesting; selector |
| `config.localOverridesInCi` | Boolean, false | Administrative only; cannot be set through user/repo/CLI overlays |
| `config.allowedProfiles` | Optional set of names; absence means unrestricted; empty means no named profiles | Administrative only; applies to selection, not authority; special value `@none` permits no-profile selection |
| `tools.allowed` | Optional set of canonical tool IDs | Administrative only; absence unrestricted; empty denies tool acquisition |
| `tools.catalogs` | Ordered catalog IDs, built-in public catalog in standalone | Administrative only; managed values reference approved catalog connectors; not arbitrary script URLs |
| `registries.routes` | Protocol/scope to connector-ID records | Administrative only; platform resolves connectors and broker credentials |

Administrative-only settings are meaningful in protected local policy even without a platform. Standalone built-ins supply public sources; a protected local administrator can restrict them. Ordinary users may choose a remote cache destination subject to constraints, but cannot invent a managed connector or credential. Additional settings must register before becoming executable behavior.

`registries.routes` entries have `{protocol, scope, connectorId}`: protocol is `npm`, `pypi`, `maven`, `cargo`, `go`, `oci`, or `tools`; scope is a protocol-validated namespace or `*`; connectorId is opaque. An exact applicable namespace wins over `*`; ambiguous equally specific matches fail. Protocol adapters own normalization and cannot fall back to public sources after an enforced route fails. Connector endpoints, authentication and token issuance remain the connector/broker contracts.

## JSON policy entries

Administrative files and signed payloads use a `settings` object keyed by canonical setting paths. Each entry supports only the following fields:

| Field | Meaning |
| --- | --- |
| `default` | Ordinary default, still overridable |
| `locked: true`, `value` | Exact enforced value; also supplies the default at that administrative layer |
| `allowed` | Nonempty finite array of allowed whole values, compared after type normalization |
| `minimum`, `maximum` | Inclusive numeric bounds, only on registered numeric settings |
| `required` | Array of mandatory members, only for set-valued settings |

An entry must contain a default or a constraint. `value` requires `locked: true`; `locked: false` is invalid (use `default` instead). `default` and `value` are mutually exclusive. Unknown entry fields/operators are errors. Constraints must be supported by the descriptor and operands must pass the same type validator as ordinary values. A policy's own default must satisfy its own constraints.

`allowed` compares whole normalized values, not substring patterns, regexes or version expressions. For example, allowed tool requests `["22", "24"]` permit precisely those selectors; resolved tool versions must additionally satisfy the selected request, lock and approved catalog. Organizations needing version-advisory filtering implement it in catalog policy instead of embedding a second version language here.

```json
{
  "schemaVersion": 1,
  "kind": "local-admin-policy",
  "settings": {
    "cache.remote": {"locked": true, "value": "oci://registry.example.test/oyzu/cache"},
    "cache.write": {"default": false, "allowed": [false]},
    "build.jobs": {"default": 8, "minimum": 1, "maximum": 16},
    "checks.coverageMinimum": {"default": 80, "minimum": 80},
    "checks.required": {"required": ["tests", "coverage"]},
    "config.localOverridesInCi": {"locked": true, "value": false}
  },
  "profiles": {},
  "requiredCapabilities": ["policy.constraints/v1"]
}
```

Mandatory set members are added to the effective set even when an ordinary array replaces lower defaults. Explain identifies their administrative origins. Ordinary configuration cannot remove required members; attempted explicit removal or an incompatible locked set fails. This is the only additive policy operator and does not change ordinary array replacement semantics.

## Composition and failure

Across administrative authorities: equal locks combine; unequal locks fail; allowed sets intersect; minimum takes the greatest bound; maximum takes the smallest; required sets union. Empty allowed intersections and reversed bounds fail. A lock must satisfy every bound, allowed set and required member. Policy composition never uses last-writer-wins for restrictions. The platform must flatten its organization/team hierarchy into one applicable snapshot before returning it.

Defaults then participate at their normal cascade positions. Requested values outside effective constraints fail with `CONFIG_OVERRIDE_DENIED`; conflicting administrative constraints fail with `CONFIG_POLICY_CONFLICT` and safe source IDs. No silent fallback, substitution or narrowing of a requested tool version is permitted.

An unknown setting with only a `default` warns and is ignored. An unknown setting containing any constraint blocks affected execution. Unknown check IDs in a mandatory set also block. Required capabilities protect new mandatory semantics. There is no inference that a field named `secure` or `locked` in ordinary TOML has authority.

## Controls that are not preferences

Organization identity, enrollment, platform endpoint, policy trust keys, policy revision, token audiences, credential values, verified source identity, signing authority and production-publication permission cannot be ordinary settings. They belong to protected bootstrap, verified evidence, broker state and authorization responses. Users may request publication; a request is not permission. Endpoint/cache redirects supplied through flags or environment must obey the same administrative constraints.

Local administrative and corporate administration can enforce trusted tool catalogs, registry routes, eligible tools, cache destinations, local-override eligibility, resource ceilings, checks, and coverage floors. Developers can normally adjust color, concurrency within bounds, development environment, tasks within builder contracts, and approved tool requests. Enforcement must cover all input channels, including native-manager adapter options, rather than only TOML parsing.

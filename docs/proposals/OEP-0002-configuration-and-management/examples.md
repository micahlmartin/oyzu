# Configuration design examples and verification cases

These are proposed input/output contracts, not evidence that the CLI implements them. Examples deliberately use reserved example domains and contain no credentials. [Resolution](implementation.md), [profiles](profiles.md), [settings](settings.md), and [managed protocol](managed-policy.md) define their semantics.

## Standalone developer

User `config.toml`:

```toml
[tools]
node = "24"
python = "3.13"

[build]
jobs = 8

[ui]
color = "auto"
```

Workspace `oyzu.toml`:

```toml
[compatibility]
schema_major = 1
requires = ["config.cascade/v1", "config.profiles/v1"]

[profile]
default = "dev"

[tools]
python = "3.12"

[env]
LOG_LEVEL = "info"

[profiles.dev.env]
LOG_LEVEL = "debug"

[profiles.integration.env]
TEST_SUITE = "integration"

[tasks.preview]
run = "python -m http.server"
```

Ignored root `oyzu.local.toml`:

```toml
[build]
jobs = 4

[env]
LOG_LEVEL = "trace"
```

Locally, Python resolves to 3.12, Node to 24, jobs to 4 and LOG_LEVEL to trace. The selected profile is dev. No account or policy server is required. Native Python requirements and the lock must agree before execution. In CI the user and local build settings are excluded: Python remains 3.12, Node is not inherited from the user file, jobs uses the detected built-in default, and dev still selects debug because the repository explicitly chose that default. Selecting dev grants no local-context exemption.

Optional minimal `build.yaml` remains unchanged:

```yaml
api:
  uses: python/app
```

## Corporate defaults and enforced constraints

The following is the settings portion of a decoded, verified server payload, not a complete signed snapshot:

```json
{
  "settings": {
    "tools.allowed": {"locked": true, "value": ["node", "python", "go", "rust", "java"]},
    "tools.catalogs": {"locked": true, "value": ["corporate-tools"]},
    "registries.routes": {"locked": true, "value": [
      {"protocol": "pypi", "scope": "*", "connectorId": "artifactory-python"},
      {"protocol": "npm", "scope": "*", "connectorId": "artifactory-npm"}
    ]},
    "cache.remote": {"locked": true, "value": "oci://registry.example.test/oyzu/cache"},
    "build.jobs": {"default": 8, "maximum": 16},
    "checks.coverageMinimum": {"default": 80, "minimum": 80},
    "checks.required": {"required": ["tests", "coverage"]},
    "config.localOverridesInCi": {"locked": true, "value": false}
  },
  "profiles": {
    "dev": {"build": {"jobs": 4}}
  },
  "offline": {"localBuilds": true, "maxAgeSeconds": 86400}
}
```

A user's jobs 12 succeeds; jobs 32 fails and identifies the maximum's policy origin. A project coverage floor of 90 succeeds; 70 fails. Replacing checks with an empty list does not remove required tests/coverage. A different cache destination fails. Native pip/npm requests use the locally configured broker and approved connector routing; no upstream credentials appear in this JSON. The connector and agent specifications own the actual transport and secret issuance.

The same enforceable cache/resource/check settings can live in a protected standalone `admin-settings.json` as shown in [settings](settings.md). Corporate management adds remote distribution and authorization, not a different CLI or evaluator. A standalone administrator must provision usable local connector definitions before selecting routes requiring them.

## Compatibility and removals

```toml
[compatibility]
requires = ["config.removal/v1"]

[overrides]
remove = ["env.DEBUG_FLAG", "tasks.preview"]

[future_preferences]
compact_table = true
```

A client supporting removal but not future_preferences warns about the optional unknown field and proceeds. A client lacking config.removal/v1 fails before executing. A strict validation command fails on the unknown preference. A typo in a known numeric value still fails ordinary validation. Unsetting a local assignment reveals inherited configuration; a removal tombstone deliberately removes inherited explicit configuration. It does not suppress an implicit builder task.

## Acceptance matrix for implementation

| Scenario | Expected result | Criteria |
| --- | --- | --- |
| Machine/user/project/nested/local/CLI precedence, including base/profile at every layer | Deterministic values and complete origins | CFG-01, CFG-04, CFG-08 |
| Two selected sibling targets with different nested files | No cross-target leakage; one profile name | CFG-04, CFG-08 |
| Whole task override omits old command environment | No accidental old-body inheritance; required reports retained | CFG-03, CFG-18 |
| Array replacement, empty array, removal and later reintroduction | Documented merge and tombstone behavior | CFG-10 |
| Unknown optional key plus lossless edit | Warning, unchanged unknown content and comments | CFG-06, CFG-14 |
| Unsupported capability, known wrong type, duplicate key, unsupported major | Error before affected execution | CFG-06 |
| Unknown enforced setting or operator | Fail closed; optional unknown default can warn | CFG-15 |
| Equal/different locks, intersecting/disjoint allowed sets, reversed bounds | Restrictive composition or explicit policy conflict | CFG-09 |
| User changes endpoint, enrollment, route or mandatory setting through env/CLI/profile | No authority override | CFG-02, CFG-09, CFG-16 |
| Standalone with/without local admin policy | Works without platform login; local restrictions apply | CFG-07 |
| CI variable spoofing or production profile | No additional artifact authority | CFG-05 |
| Valid policy at 23 hours, exactly 24 hours, shorter expiry | Allowed only strictly before the effective deadline | CFG-12 |
| Cache missing, truncated, replayed across org/user/workspace, expired, wrong signature | No public fallback | CFG-02, CFG-11 |
| Clock rollback and missing high-water state | Online reconciliation required | CFG-11, CFG-12 |
| Server revokes while old snapshot still valid | Online denial cannot be treated as transport failure | CFG-11, CFG-13 |
| Refresh during build requires extra scan | Existing plan unchanged; privileged action blocks/replans | CFG-13 |
| Config inspection with credentials, refs or arbitrary env strings | Redaction across text/JSON/errors | CFG-14, CFG-17 |
| Profile changes only display color or name | Computation cache identity unchanged; provenance records selection | CFG-17 |
| Native runtime/lock incompatible with chosen profile | Fail without silent lock rewrite | CFG-18 |
| Windows ACL/parent replacement, Unix ownership, link races | Protected source rejected when insecure | CFG-16 |
| Concurrent edit/refresh, crash, disk full | No lost edit or partially active snapshot | CFG-11, CFG-14 |

Run documentation structure checks and parse all example code blocks now. Implement the behavioral cases against the real resolver/agent later, with signed fixtures and OS-specific tests. Do not implement an example-only resolver to manufacture passing outcomes.

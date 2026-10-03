# Account access context and permission checks

Draft normative companion to [OEP-0016](README.md). These are proposed public client contracts, not accepted APIs or claims of deployed support. This page stands alone for client implementation and conformance. It defines membership context, safe access checks and the small administrative surface needed to demonstrate scoped access; it does not define private service storage, authorization-engine topology or operator procedures.

## Concepts and boundaries

Authentication identifies a subject. A human's account Membership is the account principal; the same User may have independent memberships in several accounts. Active membership alone grants no resource actions. Automation uses its own account-bound principal; credentials are not separate principals.

A role revision is a finite registered action set. A resource-group revision is a finite union of target selectors. A binding joins a principal to one exact revision of each. A request needs an action and resource match within the same eligible binding. Separate direct/group bindings union complete grants; they cannot combine unrelated actions and targets. Definition publication alone never changes a pinned active binding.

Account → Organization → Project → Component is the proposed scope hierarchy. Descendant selectors are explicit, never inferred from role name. Resource identifiers are opaque and account-bound. A public resource-access allow does not bypass feature policy, evidence or business validation and is not a bearer capability. Tool selection/acquisition and credential leases retain their separate contracts.

## Common request and response rules

Proposed routes use `/v1`. Requests authenticate through the separately versioned authentication contract. Path account must match authorized membership/context; a header or body cannot switch authority. Browser clients use the BFF session path; headless clients use the advertised API authentication method. Never send identity-provider administration credentials.

Mutations use an `Idempotency-Key`; key reuse with another canonical payload conflicts. Updates use `If-Match` on the returned ETag. IDs and revisions are opaque strings, timestamps are UTC RFC 3339. Reads paginate with opaque cursors, filter before counts/pages, and expose no inaccessible ancestor names. Requests carry `request_id`; responses echo it and provide safe errors. Unknown mandatory fields/action semantics are rejected, not ignored.

Example identifiers, endpoints and tokens below are synthetic. JSON examples describe fields, not production credentials or supported-host claims.

## Context and introspection

`GET /v1/access-context` returns the authenticated subject and its currently active account memberships: `protocol` (`oyzu.access/1`), `request_id`, `subject`, and paginated `memberships` containing `membership_id`, `account_id`, `status`. This narrowly defined authenticated self-discovery endpoint grants no scope content or action. It never lists another user's memberships. Account selection rechecks the target account's authentication requirements; challenge rather than silently reusing insufficient authentication.

`GET /v1/accounts/{account}/scopes` lists only scopes the actor may read. An empty result is valid for a new member without bindings. `GET /v1/accounts/{account}/scopes/{scope}` checks `scope.read`; `PATCH` checks `scope.update` for metadata only. Creation through `POST .../scopes` checks `scope.create` on the existing parent; body names `parent_id`, `type`, `display_name`. It cannot move a scope by changing parent through a metadata update. Dedicated move/lifecycle routes require their own reviewed contracts before exposure.

`GET /v1/accounts/{account}/permission-definitions` exposes the registered public action names, supported target types, catalog revision and safe description. This is authenticated account metadata, not evidence the caller holds any listed action. A new catalog action never silently expands a role. No customer executable permission registration or wildcard action is accepted.

## Permission check

`POST /v1/accounts/{account}/access-checks` checks the caller by default. Self checks require `access.explain-self` at the requested target; a check for another principal requires `access.explain` and server-approved inspection of that principal. A principal selector is simulation only and never becomes authentication or an executable capability. If explanation authority is absent, return `ACCESS_CHECK_FORBIDDEN`, not information about hidden bindings.

```json
{
  "protocol": "oyzu.access/1",
  "request_id": "req_example",
  "action": "scope.read",
  "resource": {"type": "project", "id": "prj_claims"}
}
```

Response fields are `protocol`, `request_id`, `decision_id`, `decision` (`allow` or `deny`), `action`, `resource`, `account_id`, `access_revision`, `reason_codes`, and optional `explanation`. Explanations may include readable binding, role-revision and selector-revision references. Missing explanation details do not mean no binding exists; they may be redacted. Unavailable authoritative state is HTTP 503 with `ACCESS_UNAVAILABLE`, not a successful allow/deny payload. Authentication/target/shape failures use errors below.

The result describes current resource permission only. Clients MUST NOT cache it as execution authority, replay decision IDs to bypass checks, or infer business success. The actual operation reauthorizes. New server authorization after acknowledged membership/group/binding revocation cannot use stale permission state. This does not undo effects already authorized or change separately issued capability expiry contracts.

## Proposed administrative slice

All records returned below are non-secret, account-scoped and revisioned. All grants use server-validated references. The server checks administration permission, grantable scope/lifetime, and any review requirement; changing a group, definition or scope cannot bypass those checks. A 202/pending response is never active access.

| Route suffix under `/v1/accounts/{account}` | Method / proposed fields | Meaning |
|---|---|---|
| `/memberships` | POST `{invited_identity}` | Invite intended verified identity; no automatic resource grant |
| `/memberships/{membership}/acceptance` | POST `{invitation_reference}` | Intended authenticated recipient accepts valid invitation; cannot accept for another user |
| `/groups` | POST `{display_name}` | Create account group; no nested groups |
| `/groups/{group}/members/{membership}` | PUT / DELETE | Add/remove account membership edge, with resulting-grant checks |
| `/roles` | POST `{display_name}` | Create draft role definition |
| `/roles/{role}/revisions` | POST `{actions}` | Immutable finite action-set revision; unknown/forbidden actions reject |
| `/roles/{role}/revisions/{revision}/publication` | POST | Publish selectable revision, not migrate bindings |
| `/resource-groups` | POST `{display_name}` | Create draft target definition |
| `/resource-groups/{group}/revisions` | POST `{selectors}` | Immutable selectors described below |
| `/resource-groups/{group}/revisions/{revision}/publication` | POST | Validate and make selectable, without live binding change |
| `/role-bindings` | POST binding request below | Validate and activate or return pending change |
| `/role-bindings/{binding}` | GET / DELETE | Read authorized summary / revoke with retained history |
| `/role-bindings/{binding}/replacements` | POST new binding revisions | Atomic replacement when activated; failure preserves predecessor |
| `/access-operations/{operation}` | GET | Authorized pending/active/failed state and safe reason |

Record creation returns ID, revision, ETag and state. Mutations that need review/dependency work return 202 with operation ID/status URL and state; direct creation returns 201. Revoke/remove returns 204 only after committed suppression; unavailable persistence returns 503, never false success. Retry does not duplicate edges/bindings; conflicting state returns 409. Pending/rejected requests grant nothing. Status inspection requires permission for the operation's affected scope and returns redacted details where needed. Full owner/review/recovery APIs are not standardized by this minimal client slice.

Selector fields are `root_scope_id`, mandatory `include_descendants`, nonempty `resource_types`, and optional nonempty exact `resource_ids`. Account must agree with all references. An absent ID filter means all otherwise matching resources; an empty filter is invalid. Descendants include future matching resources only when true; no labels/regex/programs. Exact IDs never match replacements that reuse a name.

```json
{
  "principal": {"kind": "membership", "id": "mem_alice"},
  "role_revision_id": "role_reader_rev1",
  "resource_group_revision_id": "rg_claims_rev1",
  "not_before": "2026-10-03T12:00:00Z",
  "expires_at": null
}
```

Null expiry explicitly requests indefinite access and may be rejected by administrative lifetime limits. Accepted principal kinds are membership, user-group and automation-identity. Bindings cannot target a global user, another account or mutable latest-role aliases. Replacement keeps the old binding effective until atomic activation; failure must explicitly report that fact. Removal of a single grant source does not promise total denial if other grants remain.

## Errors and client behavior

Errors contain `code`, safe `message`, `request_id`, and optionally an authorized `decision_id`; never raw tokens, upstream bodies or private evaluation topology.

| HTTP / code | Meaning / client response |
|---|---|
| 401 `AUTHENTICATION_REQUIRED` | Authenticate; do not treat as an empty membership list |
| 403 `AUTH_STEP_UP_REQUIRED` | Follow advertised account authentication challenge; no protected effect occurred |
| 403 `ACCESS_DENIED` / `ACCESS_CHECK_FORBIDDEN` | Known missing operation/inspection authority; do not retry as another principal implicitly |
| 404 `RESOURCE_NOT_FOUND` | Missing or concealed object; cannot infer which |
| 409 `ACCESS_STATE_CONFLICT` | State/approval/idempotency conflict; refresh and review, not blind replay |
| 412 `REVISION_MISMATCH` | Reload current draft/preview before submitting |
| 422 `ACCESS_REQUEST_INVALID` | Unsupported action/type/selector/reference semantics; correct request |
| 429 `RATE_LIMITED` | Honor bounded retry guidance |
| 503 `ACCESS_UNAVAILABLE` | Current evaluation/persistence unavailable; fail closed and retry boundedly |

Unavailability never produces a stale allow. Offline administrative mutation is unsupported. Context discovery and access explanations are not offline grants. Optional business capabilities remain governed by their own public validity contracts. UI must distinguish pending review, approved, activating, active, failed, expired and revoked; only active grants participate in access.

## First proof and verification examples

The following commands are illustrative until real routes/authentication are deployed. Replace placeholders in a controlled nonproduction environment; never record bearer secrets in evidence. A fixture runner should inject authentication without echoing it. With Alice assigned a scope-read binding in Claims:

```sh
curl -H 'Authorization: Bearer <alice-access-token>' \
  'https://platform.example.invalid/v1/accounts/acct_example/scopes/prj_claims'
```

Expect readable Claims metadata. A PATCH with only read authority must return 403. After an authorized administrator removes Alice's group edge and receives 204, the next GET must deny unless an independent binding still grants read. Record redacted real requests/results separately; these examples do not claim execution or define a new CLI command.

- ACCESS-PUBLIC-01: self-context lists only the authenticated user's active memberships; account switching never inherits another account's rights.
- ACCESS-PUBLIC-02: read Claims plus update Billing cannot permit update Claims; a fixture proves same-binding matching.
- ACCESS-PUBLIC-03: publication alone leaves pinned bindings unchanged; replacement failure preserves predecessor and reports it.
- ACCESS-PUBLIC-04: removing one group edge preserves any independent direct grant; acknowledged final-grant revoke blocks fresh checks/effects.
- ACCESS-PUBLIC-05: hidden resources/subjects remain concealed in lists, counts and explanations; simulated subjects cannot execute.
- ACCESS-PUBLIC-06: current-state outage yields 503/no effect; pending approval and decision IDs never act as capabilities.
- ACCESS-PUBLIC-07: expired, foreign-account, empty-selector and broader-than-authorized binding requests reject without partial effects.
- ACCESS-PUBLIC-08: public clients and synthetic conformance fixtures require no private source or documents; no private service topology or customer data appears here.

These are proposed acceptance cases, not executed tests. Real authentication, authorization and audit integration is required in addition to a public mock suite before advertising support. Proposed limits, error codes, route names and record shapes require maintainer review. This companion does not change OEP-0016's draft status or claim an access implementation exists.

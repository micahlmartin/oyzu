# Maintaining the mise fork

Normative draft companion to [OEP-0003](README.md), updated 2026-10-02.
This defines the proposed operating procedure and its implementation gates.
It does not claim that monitoring, branch protection or production integration
already exists. Maintainer acceptance of this OEP remains pending.

## Ownership and source of truth

Maintain the public `oyzuai/mise` fork for the embedding changes required by
Oyzu. The experiment established that the tested unmodified upstream library
does not expose all required seams. A fork and a dependency pin serve different
purposes: the fork holds our changes; the pin selects the exact version we build.
This is an engineering decision, not a requirement imposed by the MIT license.

@micahlmartin is the initial technical owner for update triage, patch scope,
promotion and compliance-policy changes. Legal approval remains a separate
recorded responsibility. Before required review becomes enforceable, designate
another authorized human for changes authored by that owner; self-approval or
AI approval does not satisfy independent review. An update record names its
implementer and independent reviewer.

| Location | Responsibility |
| --- | --- |
| `jdx/mise` | Upstream source, releases and advisories |
| `oyzuai/mise`, protected `main` | Reviewed integration history and Oyzu embedding patches |
| Fork `upstream/<version>-<short-sha>` branches | Disposable update candidates based on current fork main |
| Oyzu `Cargo.toml` | Public fork Git URL, full 40-character `rev`, explicit feature selection |
| Oyzu root `Cargo.lock` | Resolved dependency graph actually used to build Oyzu |
| Oyzu `tooling/mise-upstream/UPSTREAM.toml` | Upstream base, fork pin, patch and build provenance |

These paths beyond the existing fork are implementation deliverables, not files
already populated. Production uses the fork as a Git dependency initially.
Do not track a moving branch or tag in place of `rev`. The fork's Cargo.lock
does not substitute for Oyzu's application lockfile. Library targets alone are
linked and packaged; no separate mise executable is built or invoked.
Release source archives must retain the exact fork source and applicable
notices. Any later switch to Cargo vendoring must preserve identical provenance
and pass the same gates; a second independently edited vendor tree is forbidden.

The tested upstream base is `da0db43e9398b46bafa95232014708a51120e731`.
The fork was created from a different, later upstream revision. Fork creation
alone does not qualify that revision. Initial integration must either reproduce
the tested base with the reviewed patches or qualify the complete intervening
delta using this procedure before selecting its first production pin.

## Cadence and response targets

Once this process is activated, the owner performs the following checks. These
are operating targets, not a claim that a scheduler is currently configured.

| Trigger | Required action and target |
| --- | --- |
| Weekly, first business day | Review upstream releases, security advisories, dependency alerts and changes touching our patched files or admitted backends; record the checked revision/date even when no update is needed |
| Monthly, first business week | Open a routine candidate for the latest suitable stable release; record reasons for skipping releases; seek promotion within ten business days, subject to all gates |
| Relevant critical advisory or known exploitation | Triage within one business day of awareness; contain affected behavior immediately when feasible and start an expedited candidate |
| Other relevant security advisory | Triage within two business days; assign an owner and dated remediation plan based on exposure |
| More than 30 calendar days since successful promotion while newer stable releases exist | Record a maintenance exception with cause, exposure, owner and next review within seven days; review weekly until resolved |

Never update simply to make the version number current. A blocked promotion
must identify the failing gate and next action. Newly published upstream tools
or backends remain disabled until explicitly admitted. Monitoring is read-only;
automation may prepare candidates and reports but must not approve or merge.
If monitoring fails, show the last successful check and treat it as overdue.

## Required provenance and patch register

TM-02 implements a versioned schema and validator for `UPSTREAM.toml` and the
fork's `oyzu/patches.toml`. Required provenance fields are schema version,
upstream URL/version/full commit, fork URL/full commit/tree, patch-register
SHA-256, feature set, embedding ABI, registry/plugin pins, compiler/target
matrix, update-record path and previous production fork pin. The root Cargo.lock
SHA-256 is stored in the release manifest after lock regeneration. Do not put a
fork commit's own hash inside that commit; the consuming Oyzu repository binds
it. Manifest values must agree with Cargo metadata and the fetched source tree.

Every logical fork patch has a stable ID, purpose, upstream base references,
owned source paths, introducing commits, regression-test references, owner,
upstreamability decision and status (`carried`, `upstreamed` or `retired`).
An upstreamed entry records the upstream commit and removal evidence. Retired
entries retain their history. Generate a reviewable diff from the recorded
upstream base to the candidate fork tree; every product-source difference must
map to a patch entry. Separately inventory fork governance/workflow changes.
Do not maintain both hand-edited patch files and fork commits as competing
sources of truth. Exported patches are generated evidence only.

Keep patches narrow: embedding configuration, frontend identity, mediated
transport, environment behavior and explicit acquisition/layout seams. Keep
Oyzu configuration, lockfiles, policy and store orchestration in Oyzu. Propose
general-purpose seams upstream when appropriate, but submitting upstream
messages or PRs requires explicit authorization. Upstream acceptance is not a
prerequisite for a safe local fix.

## Routine update procedure

1. **Open an update record.** In the consuming Oyzu change, create
   `tooling/mise-upstream/updates/<date>-<short-upstream-sha>.md`. Record previous
   and candidate upstream/fork commits, release/advisory links, owner, reviewer,
   target date, changed backends and current disposition. State which upstream
   releases are included or skipped and why.
2. **Prepare the fork candidate.** Fetch `jdx/mise` as the `upstream` remote;
   resolve the chosen stable release to its full commit. Verify its relationship
   to the recorded base and review unusual history or tag movement. Create a
   candidate branch from fork main and merge that exact upstream commit. Do not
   force-push or rebase published main. Preserve authorship and notices. Resolve
   each conflict with a patch-register entry and regression evidence; never
   select ours/theirs wholesale merely to get a clean merge.
3. **Reconcile patches and workflows.** Remove redundant patches only after
   testing upstream's replacement against our regression cases. Refresh the
   register and inspect the complete tree diff, including build scripts,
   subprocess/network paths, trust roots, registry data and feature defaults.
   Reconcile inherited workflows into the fork's archived workflow area so an
   upstream merge cannot activate upstream publishing or deployment jobs.
4. **Review licensing and dependencies.** Preserve notices; inspect added,
   removed and changed dependencies, selected license alternatives and shipped
   feature/target graphs. Refresh factual inventories and release notices/SBOM
   through the reviewed compliance process. An updated inventory or green
   baseline check is not legal clearance. Unresolved obligations block release;
   a security deadline cannot waive them.
5. **Run fork and integration qualification.** Open a fork PR to main with its
   patch inventory, relevant tests and the draft Oyzu consumer PR linked. The
   consumer PR pins the candidate commit for testing, updates root Cargo.lock
   and provenance together, and contains all evidence in the gate table below.
   Inspect every lockfile change; do not perform an unrelated blanket update.
6. **Review and merge the fork.** Independent human approval and required checks
   must cover the current head. Merge without rewriting published history.
   Record the resulting full main commit; a squash or merge can change identity.
7. **Promote the consumer pin.** Update the Oyzu PR to that exact final fork
   commit, regenerate affected identities and rerun final qualification against
   it. Approval of a previous candidate does not approve the new head. Merge
   Oyzu only after all gates pass. Fork main advancing alone never updates Oyzu.
8. **Release and retain.** Bind source/dependency/feature identities, SBOM,
   notices, test evidence and rollback pin in the release manifest. Retain source
   archives and evidence for every distributed release. Close the update record
   with both PRs, final commits, qualification runs and remaining disabled tuples.

## Promotion gates

All gates below are mandatory for a routine promotion. Historical experimental
results cannot replace results from the proposed production integration.

| Gate | Required evidence |
| --- | --- |
| Source identity | Exact final fork commit/tree, complete patch attribution, Cargo rev/lock/provenance agreement and public clean-checkout reproducibility |
| Configuration and lock ownership | Oyzu TOML/oyzu.lock authoritative; hostile ambient mise files/settings inert; old lock compatibility explicitly tested or a visible relock required |
| Behavior | All advertised backend/platform/shell cells in the implementation matrix; selection, switching, exec, shims, activation, cancellation and concurrent store lifecycle |
| Acquisition security | Real broker/executor bypass tests, corporate proxy routing, no fallback, no exposed credentials, receipt tampering and cached-identity rejection |
| Dependency and license review | Actual shipping graph and notices/source obligations reviewed; no unresolved release-policy block |
| Performance and packaging | OEP benchmark thresholds; one delivered Oyzu executable, no separate mise executable; complete release identities |
| Quality and rollback | Required Rust/CLI checks in repository instructions, patch regression tests, previous supported pin rollback rehearsal and lock/store compatibility results |

Document exact commands, runner/OS/architecture/toolchain identities, run links,
pass/fail counts and preserved failure evidence. An unavailable platform runner
blocks promotion for a release that advertises that platform; do not convert
missing evidence into a pass. Existing failures remain tracked and cannot be
silently grandfathered into a production release.

Branch protection must require the relevant qualification and compliance checks,
current-head independent human approval, stale-review dismissal and no force
push/deletion on main in both repositories. Changes to checks, ownership or
policy require review under the existing compliance rules. Configure these
controls before declaring the process enforced; checked-in workflow files alone
do not provide server-side enforcement.

## Security fixes, failed updates and rollback

For an urgent fix, branch from the currently shipped fork revision and
cherry-pick the smallest applicable upstream fix with source attribution. Record
the advisory, exposure, commit and why a full upstream update is deferred.
Use the same two-PR promotion and all mandatory safety/license/platform gates;
parallelize preparation, not approval. If qualification cannot finish safely,
disable the affected capability through a reviewed release or advise affected
users on a verified mitigation. Do not silently enable direct downloads or
relax corporate policy to restore functionality. Merge the fix into fork main
and reconcile it during the next routine update so it is not lost or duplicated.

A failed candidate leaves the existing production pin unchanged. Rollback is
a new Oyzu PR restoring a previously qualified source and dependency set, not
a force-push or moved tag. Restore the corresponding provenance/features and
verify it with the current Oyzu facade. Never roll back to a known vulnerable
revision without a reviewed exposure/mitigation decision. Releases must refuse
unknown backend descriptors and receipts; they must not silently reinterpret
or rewrite user locks. If the previous binary cannot read a new schema, use a
forward fix or an explicitly reviewed migration, not an automatic downgrade.
Retain old payloads under existing leases and preserve user configuration.

## Implementation and acceptance

TM-02 delivers provenance schemas/validation, the patch register and a public
pinned library build. TM-12 delivers monitoring/reporting, required checks,
server-side protection, retention and a documented update/rollback rehearsal.
Monitoring credentials are read-only; candidate preparation has narrowly scoped
branch/PR permissions and no merge or publication authority.

MISE-15 is complete only after a real upstream update traverses both repositories
with current-head human review and all gates, an intentional provenance mismatch
and notice change are rejected, and rollback is demonstrated without changing
project TOML or silently rewriting oyzu.lock. Record the initial qualified pin,
reviewer assignment, last upstream check, next check and unresolved exceptions.
Until then this is a specified process, not an operational guarantee.

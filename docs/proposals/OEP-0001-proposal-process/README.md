---
id: OEP-0001
title: Enhancement proposal process
status: draft
implementation: not-started
updated: 2026-10-01
authors: [micahlmartin]
reviewers: []
requires: []
tracking-issue: null
---

# Enhancement proposal process

> Design draft for review. MUST and SHOULD express proposed normative requirements, not shipped behavior. Example syntax and protocol fields are provisional unless identified as an agreed product constraint.


## Problem and scope

Oyzu needs durable design records that work for community contributors and LLM implementation. Chat history is not an authoritative specification. This process governs significant behavior, public interfaces, compatibility, and trust-boundary changes without requiring proposals for routine fixes.

## Document types and authority

Vision documents state product direction. OEPs propose changes and record rationale. Reference documents describe implemented, supported contracts. Issues track delivery; PRs change documents or code. Accepted proposals do not imply shipped features. When accepted intent and implementation differ, record the discrepancy and fix or amend it; do not silently rewrite expectations.

The public repository holds platform vision and public behavior contracts. The private platform repository holds proprietary implementation. A public task's acceptance criteria MUST NOT require a private document. Private proposals may link public documents; public documents must stand alone.

## Identifiers and metadata

Use immutable sequential `OEP-NNNN` identifiers for public proposals and `OEP-E-NNNN` for private proposals. Maintain separate indexes. IDs are allocated by maintainers before merge, not derived from issue numbers. Never reuse an ID. Contributors start with `OEP-draft`; the initial IDs in this documentation set are allocated as drafts.

Each proposal has one directory and a README with front matter: id, title, status, implementation, updated, authors, reviewers, requires, and tracking-issue. Dependencies use stable IDs. A null tracking issue means no GitHub issue exists yet. Authors reflect the human product sponsor; generated prose does not make an agent a project approver.

## Lifecycle and review

Design states: draft, in-review, accepted, rejected, withdrawn, superseded. Accepted designs may be superseded by a linked successor. Delivery states: not-started, in-progress, experimental, stable. Rejected proposals are retained with rationale.

Initially micahlmartin is the accountable maintainer. Acceptance requires a recorded maintainer review. Approval of the general process does not automatically accept every technical choice drafted here. Material changes to accepted contracts need reviewed amendments or a successor; editorial corrections can be ordinary PRs.

## Implementation workflow

Open a problem issue; determine whether an OEP is warranted; submit a design PR; resolve blocking questions; record acceptance; create bounded implementation issues; implement with linked verification. Bug fixes within an existing contract need no new OEP. Cross-repository changes use linked issues and compatible protocol rollout.

LLM tasks receive the accepted contract, scope, acceptance criteria, and relevant repository guidance. They may prepare experiments for open questions but MUST NOT mark a design accepted or a feature stable themselves.

## Template and graduation

Required subjects are problem, goals/non-goals, user experience, contracts, state/flow, security, failure handling, platform behavior, performance, verification, rollout, alternatives, and open decisions. A proposal may combine related sections or mark a genuinely irrelevant topic with justification.

The maintainer's [end-to-end-first implementation priority](../../decisions.md#end-to-end-first-implementation-priority)
also requires a concrete proof of the intended solution: real inputs, integration
path, runnable scenario and observed result. Make this the first implementation
milestone, then expand scope and harden in explicit stages. Clearly separate
prerequisites for responsible proof execution, mandatory release requirements and
optional robustness improvements. Component tests alone do not prove a proposal.

Stable graduation requires demonstrated acceptance criteria, compatibility documentation, user reference updates, and relevant OS coverage. A private GitHub Project may summarize delivery across repositories; public issues remain sufficient for community work.

## Acceptance criteria

- DOC-01: every ID is unique and dependency IDs resolve.
- DOC-02: every proposal has design and delivery status; draft content is not represented as implemented reference.
- DOC-03: public relative links never point into the private repository.
- DOC-04: significant implementation PRs link design and verification evidence.
- DOC-05: rejected/superseded designs remain discoverable.

## Alternatives and open decisions

A separate enhancements repository is unnecessary initially. A parallel mandatory ADR system would duplicate decisions. Committee/SIG governance can be added when the maintainer community needs it.

## Source

The structure borrows tracked design review from the [Kubernetes KEP process](https://github.com/kubernetes/enhancements/blob/master/keps/README.md); Oyzu's lifecycle and numbering here are its own proposal.

# Oyzu Enhancement Proposals

All proposals below remain **draft**. Implementation has begun for configuration, discovery, tasks and the initial captured-source build/bundle path; see each proposal metadata and the [measured status](../implementation-status.md). No proposal has been accepted by a maintainer yet. IDs are permanent and independent of GitHub issue numbers.

| ID | Proposal | Dependencies |
| --- | --- | --- |
| OEP-0001 | [Enhancement proposal process](OEP-0001-proposal-process/README.md) | None |
| OEP-0002 | [Configuration and managed settings](OEP-0002-configuration-and-management/README.md) | OEP-0001 |
| OEP-0003 | [In-process mise integration](OEP-0003-mise-integration/README.md) | OEP-0002 |
| OEP-0004 | [Tool acquisition and locking](OEP-0004-tool-acquisition-and-locking/README.md) | OEP-0002, OEP-0003 |
| OEP-0005 | [Task discovery overrides and hooks](OEP-0005-tasks-and-hooks/README.md) | OEP-0002 |
| OEP-0006 | [Builder discovery and build planning](OEP-0006-discovery-and-planning/README.md) | OEP-0004, OEP-0005 |
| OEP-0007 | [Source capture and hermetic execution](OEP-0007-hermetic-execution/README.md) | OEP-0006 |
| OEP-0008 | [Environment activation and shell integration](OEP-0008-environment-and-shells/README.md) | OEP-0002, OEP-0003 |
| OEP-0009 | [Agent lifecycle and local package proxy](OEP-0009-agent-and-local-proxy/README.md) | OEP-0002, OEP-0010 |
| OEP-0010 | [Connector and credential contracts](OEP-0010-connectors-and-credentials/README.md) | OEP-0002 |
| OEP-0011 | [Local and OCI remote build caching](OEP-0011-oci-build-cache/README.md) | OEP-0006, OEP-0007 |
| OEP-0012 | [Build bundles manifests and reports](OEP-0012-build-bundles/README.md) | OEP-0006 |
| OEP-0013 | [Versioning release eligibility and publication](OEP-0013-release-and-publication/README.md) | OEP-0010, OEP-0012 |
| OEP-0014 | [Ecosystem builders and executable examples](OEP-0014-builders-and-examples/README.md) | OEP-0005, OEP-0006, OEP-0007, OEP-0012 |
| OEP-0015 | [Desktop integration and distribution](OEP-0015-desktop-and-distribution/README.md) | OEP-0009 |
| OEP-0016 | [Public platform protocol and policy decisions](OEP-0016-platform-protocol/README.md) | OEP-0002, OEP-0010 |

The build implementation pass adds detailed companions to OEP-0002/0004–0007/0011–0014/0016. Start with the [build implementation map](../build-implementation.md) and [draft record schemas](../contracts/README.md).

The [OEP-0003 implementation proposal](OEP-0003-mise-integration/README.md) now
includes concrete runtime, format-2 lock/store, acquisition/authorization and
delivery contracts derived from the mise experiment. It remains draft; measured
experiment success does not imply production support or design acceptance.

| ID | Proposal | Dependencies |
| --- | --- | --- |
| OEP-0017 | [Package-manager acquisition and credential isolation](OEP-0017-dependency-acquisition/README.md) | OEP-0007, OEP-0009, OEP-0010, OEP-0014 |
| OEP-0018 | [Container packaging materialization and Helm composition](OEP-0018-container-and-helm-packaging/README.md) | OEP-0006, OEP-0007, OEP-0012, OEP-0014, OEP-0017 |
| OEP-0019 | [Versioned build records and implementation conformance](OEP-0019-build-records-and-conformance/README.md) | OEP-0006, OEP-0011, OEP-0012, OEP-0013, OEP-0017, OEP-0018 |

Use the [template](TEMPLATE.md) for new proposals and follow [OEP-0001](OEP-0001-proposal-process/README.md). Contributors use a draft ID until maintainers allocate the next number. Tracking issues are currently null; this index does not claim issues exist.

Private server implementation uses a separate OEP-E series. Public contracts remain self-contained here.

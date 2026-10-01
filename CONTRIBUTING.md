# Contributing to Oyzu

Oyzu is in initial planning. The repository contains draft visions and design proposals, not a released implementation. Start with the [design library](docs/README.md).

Open an issue to discuss a bug, feature, or bounded implementation task. For significant features or contract changes, agree on the design before substantial implementation. Follow [OEP-0001](docs/proposals/OEP-0001-proposal-process/README.md) and use the [proposal template](docs/proposals/TEMPLATE.md). IDs are allocated by maintainers and are independent of issue numbers.

Design acceptance and delivery status are separate. The initial accountable maintainer is micahlmartin; generated work must not mark its own proposals accepted. Link implementation issues to acceptance criteria and example fixtures. Public work must remain independently implementable without private repository access.

Pull requests should explain the intended behavior, reference related issues or proposals, and provide relevant verification evidence. Include operating-system and shell differences where applicable.

For documentation changes, run `node tooling/check-docs.mjs` and `git diff --check`. These checks do not validate unimplemented product behavior.

Never include secrets or confidential information. See [SECURITY.md](SECURITY.md) for vulnerability reporting.

The project license and contribution licensing terms remain undecided. Please defer substantive code contributions until those terms are established.

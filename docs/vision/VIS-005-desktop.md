---
id: VIS-005
status: draft
updated: 2026-10-01
---

# Optional desktop experience

> Vision draft consolidating agreed product direction. No feature is implemented by this document. Unresolved implementation choices are identified explicitly.


The desktop app provides visibility and convenient controls for the same agent used by the CLI. It is not part of the build execution dependency chain.

## User experience

A user can see the active organization, authentication state, effective package routes, installed tools, and diagnostic failures. They can initiate supported login flows, inspect policy origins, and copy redacted diagnostic information. Administrative policy cannot be disabled from a local settings screen.

The CLI must provide equivalent essential operations. No feature may require keeping a window open, accepting an invisible GUI prompt in CI, or installing a webview on a headless server.

## Architecture

The proposed implementation is a Tauri shell with a React/TypeScript interface. A small native bridge communicates with the independent agent through authenticated local IPC. Renderer code does not receive registry credentials or unrestricted shell/file capabilities.

Desktop installation may register the same `oyzu` executable for background startup. Desktop exit, agent stop, and uninstall are distinct actions. Agent stop warns about active proxy operations; managed requirements constrain what the local user may change.

## Distribution and compatibility

Windows, macOS, and Linux packages must declare their native webview/runtime dependencies. Signing, notarization where applicable, safe updating, and rollback are packaging work rather than builder semantics.

The app negotiates agent protocol capabilities. An older app must report incompatibility rather than write settings using an incompatible schema. Updates should coordinate active operations before replacing the executable.

## Success

All build and installation acceptance tests run with the desktop app absent. Closing the UI during a package download does not terminate the agent. The UI displays effective configuration from the agent rather than independently resolving it.

## Open questions

Minimum desktop OS versions, distribution channels, update transport, tray behavior, and accessibility verification remain draft implementation choices.

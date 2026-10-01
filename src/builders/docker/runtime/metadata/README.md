# Native Docker metadata adapter

This small executable uses BuildKit's Dockerfile parser and instruction model,
plus Docker's ignore-file matcher. Rust owns builder admission, preparation,
planning and execution policy. The adapter emits facts only; an empty list of
external requirements is not proof of sandbox safety or a successful build.

Run `oyzu-docker-metadata <captured-context>` to read the conventional Dockerfile
and emit JSON. It executes no project commands and contacts no services. It
reports native stage bases/platforms, external input requirements, sensitive RUN
mounts, deferred ONBUILD instructions and files retained by native ignore rules.
Dockerfile-specific ignore files take precedence over `.dockerignore`. Negations
within excluded directories are evaluated without pruning those directories.
Source contents and arbitrary RUN/ENV bodies are not copied into metadata.

The Rust integration must resolve or reject each requirement before executing a
build. Dynamic references need evaluated, captured inputs; they must not be
treated as resolved strings. External images/frontends/contexts, remote ADD,
secret/SSH mounts, cache provenance, platforms and deferred triggers all need
their respective admission paths. Stage-cycle validation remains BuildKit's
responsibility. This is not an independent Dockerfile interpreter.

BuildKit is pinned to v0.25.0 and patternmatcher to v0.6.0 in `go.mod`, with module
checksums in `go.sum`. Both are Apache-2.0 dependencies. No upstream source is
vendored or modified here. The toolchain image preserves linked dependencies'
license/notice files under `/usr/share/oyzu-docker-metadata/notices`; this does not
select a license for Oyzu. Upstream sources:
[BuildKit](https://github.com/moby/buildkit/tree/v0.25.0),
[patternmatcher](https://github.com/moby/patternmatcher/tree/v0.6.0).

`go test -mod=readonly ./...` checks actual native parsing, sensitive input
discovery, heredocs, malformed instructions and ignore behavior. Run offline after
provisioning module dependencies. Symlink checks require host symlink support;
Linux CI exercises them. These adapter tests are not full container scenario
acceptance.

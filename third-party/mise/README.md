# Mise development dependency

Source: https://github.com/oyzuai/mise at
1da2a9fa009ada755cbcc96e5d944fe1cd61072c.
The original upstream MIT license is preserved verbatim in LICENSE.

The maintainer authorized controlled development integration on 2026-10-02.
Cargo feature mise-integration selects the library with default features disabled
and rustls plus vendored-lua enabled. No separate mise executable is built or
invoked. Cargo.lock binds the resolved consumer dependencies.

This notice is not a complete transitive-dependency notice bundle or distribution
approval. Dependency and bundled-data licenses retain their own terms; do not
infer that mise's MIT license covers them. Release audit remains separate.

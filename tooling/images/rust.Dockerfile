# CI explicitly provisions this image. Oyzu never downloads a tool during a build.
FROM rust:1.94.0-slim-bookworm
RUN rustup component add rustfmt clippy
ADD https://github.com/nextest-rs/nextest/releases/download/cargo-nextest-0.9.146/cargo-nextest-0.9.146-x86_64-unknown-linux-gnu.tar.gz /tmp/nextest.tar.gz
RUN echo '682c21b777c333e96fd532e114d3a5a894e0729ab88d94c0a9f20f8419695428  /tmp/nextest.tar.gz' | sha256sum --check \
    && tar -xzf /tmp/nextest.tar.gz -C /usr/local/cargo/bin cargo-nextest \
    && rm /tmp/nextest.tar.gz

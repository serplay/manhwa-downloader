# Multi-stage build: compile the Rust server, ship a small Debian runtime.
# BoringSSL (pulled in by the Chrome-impersonating HTTP client) needs cmake,
# clang and perl at build time only.

FROM rust:1.88-bookworm AS builder
RUN apt-get update && apt-get install -y --no-install-recommends \
    cmake clang perl pkg-config \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app

# Cache dependencies separately from source.
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && echo 'fn main() {}' > src/main.rs \
    && cargo build --release --locked \
    && rm -rf src target/release/manhwa-server target/release/deps/manhwa_server-*

COPY src ./src
COPY assets ./assets
RUN cargo build --release --locked

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates tini \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --uid 10001 app
# CBR output needs the proprietary `rar` binary; it is not redistributable, so
# it is not installed here. Mount or bake it in yourself if you want CBR.
WORKDIR /app
COPY --from=builder /app/target/release/manhwa-server /usr/local/bin/manhwa-server
RUN mkdir -p /data/downloads && chown -R app:app /data
USER app
ENV PORT=8000 \
    BIND=0.0.0.0 \
    DOWNLOAD_DIR=/data/downloads \
    RUST_LOG=info
EXPOSE 8000
VOLUME ["/data/downloads"]
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s \
    CMD ["/bin/sh", "-c", "exec 3<>/dev/tcp/127.0.0.1/${PORT} && printf 'GET /health HTTP/1.0\\r\\n\\r\\n' >&3 && head -c 64 <&3 | grep -q ' 200 '"]
ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["manhwa-server"]

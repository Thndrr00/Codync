# Codync host in a container: one Rust binary that runs the bots and serves the apps' API.
# Not a workspace platform: bots, their agents and their data all live in this one container
# (docs/guides/docker.md#what-docker-does-not-give-you).
#
#   docker build -t codync-host:local .
#   docker run -d --name codync -p 19222:19222 -v codync-data:/data codync-host:local
#
# The production environment is compiled in (`CODYNC_ENV=main`): port 19222, `~/.codync`,
# https://api.codync.dev. A dev build would be port 19223 and the dev cloud, which a server
# image must never talk to.
ARG RUST_VERSION=1.99
ARG DEBIAN_VERSION=bookworm

FROM rust:${RUST_VERSION}-${DEBIAN_VERSION} AS build

# `CODYNC_ENV` is read with `option_env!` at compile time (host/src/environment.rs).
ARG CODYNC_ENV=main
ENV CODYNC_ENV=${CODYNC_ENV}

# The C toolchain comes with the Rust image. aws-lc-rs builds through CMake, and keyring's
# vendored OpenSSL needs Perl; without them the build stops deep in a dependency.
RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends cmake perl pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Dependencies first, as a throwaway binary, so changing host/src doesn't refetch the crates.
COPY host/Cargo.toml host/Cargo.lock host/rust-toolchain.toml host/rustfmt.toml host/clippy.toml ./host/
COPY host/crates ./host/crates
COPY host/vendor ./host/vendor
RUN cd host \
    && mkdir -p src \
    && echo 'fn main() {}' > src/main.rs \
    && cargo build --release --locked

# The real sources, plus the trees it reaches with `include_str!`/`include_bytes!`:
# host/n8n-nodes-codync, packaging/{n8n,comfy,updates} and the apps' language catalogs.
COPY host/src ./host/src
COPY host/n8n-nodes-codync ./host/n8n-nodes-codync
COPY packaging ./packaging
COPY apps ./apps
RUN cd host && cargo build --release --locked

FROM debian:${DEBIAN_VERSION}-slim AS runtime

# Stamped by `--platform`, so the image says which architecture it was built for.
ARG TARGETARCH
LABEL org.opencontainers.image.title="Codync host" \
      org.opencontainers.image.description="Runs your coding-agent bots and serves the Codync apps" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.architecture="${TARGETARCH}"

# ca-certificates is required: a host without them stops with "TLS setup failed".
# tzdata gives routines and logs the machine's zone (set TZ; otherwise it is UTC).
# curl is the health check's client.
RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        ca-certificates curl tzdata \
    && rm -rf /var/lib/apt/lists/*

# A fixed non-root user; /data is the host's CODYNC_HOME and the volume's mount point.
RUN groupadd --gid 10001 codync \
    && useradd --uid 10001 --gid codync --create-home --home-dir /home/codync --shell /usr/sbin/nologin codync \
    && install -d -o codync -g codync -m 0700 /data

COPY --from=build /build/host/target/release/codync-host /usr/local/bin/codync-host
COPY docker/entrypoint.sh /usr/local/bin/codync-entrypoint
COPY docker/healthcheck.sh /usr/local/bin/codync-healthcheck
RUN chmod 0755 /usr/local/bin/codync-host /usr/local/bin/codync-entrypoint /usr/local/bin/codync-healthcheck

ENV CODYNC_HOME=/data \
    CODYNC_BIND=0.0.0.0 \
    CODYNC_PORT=19222 \
    HOME=/home/codync \
    RUST_LOG=codync_host=info

USER codync
WORKDIR /home/codync
VOLUME ["/data"]
EXPOSE 19222

# GET /health is the host's own open route (host/src/api/http.rs); `codync-host status` always
# exits 0, so it can't answer for Docker.
HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
    CMD ["/usr/local/bin/codync-healthcheck"]

ENTRYPOINT ["/usr/local/bin/codync-entrypoint"]
CMD ["serve"]

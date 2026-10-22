# syntax=docker/dockerfile:1.28@sha256:bb22d9815c728170f72750f4e5b0d672e06176142e1d602c7e66c050100b7e5b

# ---------------------------------------------------------------------------
# Stage 1: build the React web UI with pnpm
# ---------------------------------------------------------------------------
FROM node:24-alpine3.24@sha256:06d7adf3e0ef61a13f1d28682726366f97a15aa1888c934d3ecec86c33918b53 AS web-builder

WORKDIR /build

RUN corepack enable \
    && corepack prepare pnpm@12.3.4 --activate

COPY web/package.json web/pnpm-lock.yaml web/pnpm-workspace.yaml ./
# The pnpm store lives on a BuildKit cache mount, i.e. a different filesystem
# than /build/node_modules, so packages cannot be hard-linked or cloned into
# node_modules. pnpm 11 fell back to copying automatically; pnpm 12 errors out
# ("Operation not permitted") instead, so force the copy import method.
RUN --mount=type=cache,id=pnpm-store,target=/root/.local/share/pnpm/store \
    pnpm install --frozen-lockfile --config.strict-dep-builds=false --config.package-import-method=copy

COPY web/ ./
RUN --mount=type=cache,id=pnpm-store,target=/root/.local/share/pnpm/store \
    pnpm run build

# ---------------------------------------------------------------------------
# Stage 2: build the Rust server, cli and admin binaries
# ---------------------------------------------------------------------------
FROM rust:1.99.1-slim-bookworm@sha256:86234e857a413004348b471d11a4ebe1aa92d2eae2b6b143cac68fd1ac3c48be AS rust-builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        ca-certificates \
        git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Passed as --build-arg by the teccave pipeline. MAILBOXD_VERSION must be
# visible to every crate during compilation (option_env! reads the env of
# the specific crate being built), not only to crates/server/build.rs.
ARG VERSION=v0.0.0-dev
ENV VERSION=${VERSION}
ENV MAILBOXD_VERSION=${VERSION}

COPY . .
COPY --from=web-builder /build/dist ./web/dist

RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=cargo-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=cargo-target,target=/build/target,sharing=locked \
    cargo build --release --workspace \
        --bin mailboxd-server --bin mailboxd-cli --bin mailboxd-admin \
    && mkdir -p /out \
    && cp target/release/mailboxd-server /out/ \
    && cp target/release/mailboxd-cli /out/ \
    && cp target/release/mailboxd-admin /out/

# ---------------------------------------------------------------------------
# Stage 3: minimal runtime image
# ---------------------------------------------------------------------------
FROM debian:bookworm-slim@sha256:7ff6ba31527b9e336162fa061746c366c4b2032f4d2cff2078bea43f78efe3f8 AS runtime

ARG VERSION=v0.0.0-dev
ENV VERSION=${VERSION}

LABEL org.opencontainers.image.title="mailboxd" \
      org.opencontainers.image.description="Self-hosted email archiving server built in Rust" \
      org.opencontainers.image.source="https://git.teccave.de/tecbeat/mailboxd" \
      org.opencontainers.image.licenses="AGPL-3.0-or-later" \
      org.opencontainers.image.version="${VERSION}"

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -g 1000 mailboxd \
    && useradd -u 1000 -g mailboxd -m -s /usr/sbin/nologin mailboxd \
    && mkdir -p /data /opt/mailboxd \
    && chown -R mailboxd:mailboxd /data /opt/mailboxd

COPY --from=rust-builder /out/mailboxd-server /opt/mailboxd/mailboxd-server
COPY --from=rust-builder /out/mailboxd-cli    /usr/local/bin/mailboxd-cli
COPY --from=rust-builder /out/mailboxd-admin  /usr/local/bin/mailboxd-admin
COPY LICENSE NOTICE /opt/mailboxd/

USER mailboxd:mailboxd
WORKDIR /data

EXPOSE 15630

HEALTHCHECK --interval=30s --timeout=5s --start-period=15s --retries=3 \
    CMD curl -fsS http://localhost:15630/api/status || exit 1

# The server runs the storage migration itself on startup; no wrapper needed.
ENTRYPOINT ["/opt/mailboxd/mailboxd-server"]

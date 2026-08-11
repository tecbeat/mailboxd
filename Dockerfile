# syntax=docker/dockerfile:1.26@sha256:ecfaec9ed6d810b56388c508f4121597bfbba70d41a6dfeee4d8cad5f295fc32

# ---------------------------------------------------------------------------
# Stage 1: build the React web UI with pnpm
# ---------------------------------------------------------------------------
FROM node:24-alpine3.24@sha256:d32cdf619f63fe0471182d08996dd516c6275bb5fd31ae06e55a570bd9e1ad43 AS web-builder

WORKDIR /build

RUN corepack enable \
    && corepack prepare pnpm@11.18.0 --activate

COPY web/package.json web/pnpm-lock.yaml web/pnpm-workspace.yaml ./
RUN --mount=type=cache,id=pnpm-store,target=/root/.local/share/pnpm/store \
    pnpm install --frozen-lockfile --config.strict-dep-builds=false

COPY web/ ./
RUN --mount=type=cache,id=pnpm-store,target=/root/.local/share/pnpm/store \
    pnpm run build

# ---------------------------------------------------------------------------
# Stage 2: build the Rust server, cli and admin binaries
# ---------------------------------------------------------------------------
FROM rust:1.97.1-slim-bookworm@sha256:158b745f1b82dbeec7ea06e6b1617d6b005723bb66e6141cd2ddfee40d079ec3 AS rust-builder

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
FROM debian:bookworm-slim@sha256:abd67ffcfa541b485a3dff59865ab629aa048a6c613e639d36e7456b0b229241 AS runtime

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

ENTRYPOINT ["/opt/mailboxd/mailboxd-server"]

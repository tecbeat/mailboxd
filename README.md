# mailboxd

A self-hosted email archiving server built in Rust. Downloads mail from IMAP
accounts, builds a full-text search index, and serves a REST API with an
embedded web UI. Purpose-built for long-term preservation, unified cross-account
search, and programmatic access to archived email.

**License:** GNU Affero General Public License v3.0 or later.
See [LICENSE](LICENSE) and [NOTICE](NOTICE).

**Source code:** <https://git.teccave.de/tecbeat/mailboxd>

> mailboxd is an archiver, not an email client. It does not send, compose,
> forward, or reply to emails. Its optional SMTP server is for **receiving**
> emails only.

## Contents

- [Quick start](#quick-start)
- [Configuration reference](#configuration-reference)
- [Authentication & RBAC](#authentication--rbac)
- [CLI tools](#cli-tools)
- [Architecture](#architecture)
- [Storage & backup](#storage--backup)
- [License and attribution](#license-and-attribution)

## Quick start

### Build from source

Prerequisites: Rust (latest stable), Node.js 20+, pnpm.

```bash
git clone https://git.teccave.de/tecbeat/mailboxd.git
cd mailboxd

cd web && pnpm install && pnpm run build && cd ..

export MAILBOXD_ENCRYPT_PASSWORD=dev-password
cargo run -p mailboxd-server -- --mailboxd-root-dir /tmp/mailboxd-data
```

Open <http://localhost:15630>.

> Default login: username `admin`, password `admin@mailboxd`.
> **Change this immediately** via Settings → Profile.

### Docker

The project ships a `docker/Dockerfile` and a `docker/build.sh` helper.
Container images are not yet published to a registry; build locally:

```bash
cd web && pnpm install && pnpm run build && cd ..
cargo build --release
mkdir -p docker/amd64
cp target/release/mailboxd-{server,cli,admin} docker/amd64/
cp LICENSE NOTICE docker/amd64/
docker build --build-arg TARGETARCH=amd64 --build-arg CRATE_VERSION=dev \
    -t mailboxd:dev docker/
```

Run:

```bash
mkdir -p ./mailboxd-data

docker run -d \
  --name mailboxd \
  -p 15630:15630 \
  -v $(pwd)/mailboxd-data:/data \
  --user 1000:1000 \
  -e MAILBOXD_ROOT_DIR=/data \
  -e MAILBOXD_ENCRYPT_PASSWORD=your-secure-password-here \
  mailboxd:dev
```

### Docker Compose

```yaml
services:
  mailboxd:
    image: mailboxd:dev
    container_name: mailboxd
    ports:
      - "15630:15630"
    volumes:
      - ./mailboxd-data:/data
    user: "1000:1000"
    environment:
      MAILBOXD_ROOT_DIR: /data
      MAILBOXD_ENCRYPT_PASSWORD: your-secure-password-here
      MAILBOXD_LOG_LEVEL: info
```

## Configuration reference

All settings accept both CLI flags (`--mailboxd-http-port`) and environment
variables (`MAILBOXD_HTTP_PORT`). CLI flags take precedence.

### Required

| Variable | CLI flag | Description |
|----------|----------|-------------|
| `MAILBOXD_ROOT_DIR` | `--mailboxd-root-dir` | **Required.** Absolute path for all persistent data. |
| `MAILBOXD_ENCRYPT_PASSWORD` | `--mailboxd-encrypt-password` | Password used to encrypt stored credentials (IMAP passwords, OAuth tokens). |
| `MAILBOXD_ENCRYPT_PASSWORD_FILE` | `--mailboxd-encrypt-password-file` | Alternative: read the encryption password from a file. |

### Server & networking

| Variable | Default | Description |
|----------|---------|-------------|
| `MAILBOXD_HTTP_PORT` | `15630` | HTTP server port. |
| `MAILBOXD_BIND_IP` | `0.0.0.0` | IP address to bind to (IPv4 or IPv6). |
| `MAILBOXD_PUBLIC_URL` | `http://localhost:15630` | Public-facing URL used in OAuth redirects and docs. |
| `MAILBOXD_BASE_URL` | `/` | Base path for the web UI when behind a reverse proxy (e.g. `/mailboxd`). |
| `MAILBOXD_WEBUI_TOKEN_EXPIRATION_HOURS` | `168` | Access token lifetime in hours (default 7 days). |
| `MAILBOXD_HTTP_COMPRESSION_ENABLED` | `true` | Enable gzip/brotli/zstd response compression. |

### Logging

| Variable | Default | Description |
|----------|---------|-------------|
| `MAILBOXD_LOG_LEVEL` | `info` | Log level: `trace`, `debug`, `info`, `warn`, `error`. |
| `MAILBOXD_ANSI_LOGS` | `true` | Colorized terminal output. |
| `MAILBOXD_JSON_LOGS` | `false` | JSON-formatted logs for log aggregators. |
| `MAILBOXD_LOG_TO_FILE` | `false` | Persist logs to files under root dir. |
| `MAILBOXD_MAX_SERVER_LOG_FILES` | `5` | Max log files to retain. |

### CORS

| Variable | Default | Description |
|----------|---------|-------------|
| `MAILBOXD_CORS_ORIGINS` | *(allow all)* | Comma-separated list of allowed origins. |
| `MAILBOXD_CORS_MAX_AGE` | `86400` | Cache duration for CORS preflight in seconds. |

### TLS

| Variable | Default | Description |
|----------|---------|-------------|
| `MAILBOXD_ENABLE_REST_HTTPS` | `false` | Serve the API over HTTPS (requires valid certificate). |

### OpenID Connect (OIDC) single sign-on

mailboxd can delegate web UI authentication to any OIDC provider (Authentik,
Keycloak, PocketID, Authelia, Zitadel, Dex, …) using the authorization code
flow with PKCE.

| Variable | Default | Description |
|----------|---------|-------------|
| `MAILBOXD_OIDC_ENABLED` | `false` | Master switch for OIDC single sign-on. |
| `MAILBOXD_OIDC_ISSUER_URL` | — | Issuer URL. Discovery via `/.well-known/openid-configuration`. |
| `MAILBOXD_OIDC_CLIENT_ID` | — | OAuth2 client ID registered with the IdP. |
| `MAILBOXD_OIDC_CLIENT_SECRET` | — | OAuth2 client secret registered with the IdP. |
| `MAILBOXD_OIDC_REDIRECT_URI` | — | Must resolve to `<public-url>/api/auth/oidc/callback`. |
| `MAILBOXD_OIDC_DEFAULT_ROLE_ID` | `100200000000000` (Member) | Global role ID assigned to auto-provisioned OIDC users. |
| `MAILBOXD_OIDC_AUTO_REDIRECT` | `false` | Redirect `/sign-in` directly to the IdP. Local login stays reachable via `/sign-in?local=1`. |

### SMTP receiver

| Variable | Default | Description |
|----------|---------|-------------|
| `MAILBOXD_ENABLE_SMTP` | `false` | Enable the embedded SMTP receiver. |
| `MAILBOXD_SMTP_PORT` | `2525` | SMTP listening port. |
| `MAILBOXD_SMTP_ENCRYPTION` | `starttls` | `none`, `starttls`, or `tls`. |
| `MAILBOXD_SMTP_AUTH_REQUIRED` | `true` | Require authentication for SMTP connections. |
| `MAILBOXD_TLS_KEY_PATH` | — | Absolute path to SMTP TLS private key. |
| `MAILBOXD_TLS_CERT_PATH` | — | Absolute path to SMTP TLS certificate chain. |

### Storage paths

| Variable | Default | Description |
|----------|---------|-------------|
| `MAILBOXD_INDEX_DIR` | `{root}/mailboxd-indices` | Tantivy full-text index directory. |
| `MAILBOXD_DATA_DIR` | `{root}/mailboxd-storage` | Fjall blob storage directory. |

> Place `MAILBOXD_INDEX_DIR` on fast SSD storage for responsive search, and
> `MAILBOXD_DATA_DIR` on high-capacity HDD for cost-effective blob storage.

> mailboxd does **not** support writing data directly to a network file system
> (NFS, CIFS/SMB, etc.). All data directories must reside on a local file
> system.

### Performance tuning

| Variable | Default | Description |
|----------|---------|-------------|
| `MAILBOXD_SYNC_CONCURRENCY` | `num_cpus × 2` | Max concurrent account sync tasks. |
| `MAILBOXD_METADATA_CACHE_SIZE` | `134217728` (128 MB) | Metadata DB cache in bytes. |
| `MAILBOXD_ENVELOPE_CACHE_SIZE` | `134217728` (128 MB) | Envelope index cache in bytes. |

## Authentication & RBAC

1. `POST /api/login` with username and password returns a JWT access token.
2. `GET /api/auth/oidc/login` starts an OIDC single sign-on flow.
3. All `/api/v1/*` endpoints require `Authorization: Bearer <token>`.
4. Tokens expire after `MAILBOXD_WEBUI_TOKEN_EXPIRATION_HOURS` (default 7 days).
5. Long-lived API tokens can be created via the web UI or API for programmatic
   access.

### Default admin account

On first start, mailboxd creates a built-in admin user:

- **Username:** `admin`
- **Password:** `admin@mailboxd`

**Change the password immediately** via the web UI (Settings → Profile).
If locked out, use `mailboxd-admin` to reset it.

### Built-in roles

| Role | Type | Scope | Description |
|------|------|-------|-------------|
| **Admin** | Global | Unrestricted | Full system access. |
| **Manager** | Global | ACL-scoped | Create accounts, view users, manage authorized accounts and their data. |
| **Member** | Global | Minimal | Basic login access; data access granted through account-level role assignments. |
| **AccountManager** | Account | Per-account | Full control over an assigned account. |
| **AccountViewer** | Account | Per-account | Read-only access to an assigned account. |

Custom roles can be created via the web UI (`/users/roles`) or API.

## CLI tools

### `mailboxd-cli` — import & export

```bash
./mailboxd-cli --config config.toml
```

Supports EML directories, MBOX files (including Gmail's MBOX variant),
Thunderbird profiles, and Outlook PST files. Exports as MBOX.

### `mailboxd-admin` — administration

```bash
./mailboxd-admin
```

Interactive menu:

- **Reset admin password** when locked out.
- **Legacy storage migration** (v0.3.7 → v1.x) — inherited from upstream.

## API reference

Interactive API documentation is served by the running instance:

| Endpoint | UI |
|----------|----|
| `/api-docs/swagger` | Swagger UI |
| `/api-docs/redoc` | ReDoc |
| `/api-docs/scalar` | Scalar |
| `/api-docs/spec.json` | Raw OpenAPI 3.0 JSON |

All `/api/v1/*` endpoints require `Authorization: Bearer <token>`.

## Architecture

### Workspace crates

```
mailboxd/
├── crates/
│   ├── memdb/    Embedded key-value database layer (WAL, transactions)
│   ├── blob/     LZ4-compressed blob storage engine
│   ├── core/     Library — IMAP sync, search, storage, auth, models
│   ├── server/   Binary — Poem web server + embedded web UI (rust-embed)
│   ├── cli/      Binary — mailboxd-cli import/export
│   ├── admin/    Binary — mailboxd-admin password reset & migration
│   └── smtp/     Library — embedded SMTP receiver
└── web/          React + TypeScript + Vite + ShadCN UI frontend
```

### Storage layers

- **memdb** — key-value metadata store (accounts, users, roles, OAuth2
  configs, proxy settings, system configuration).
- **Tantivy** — full-text search indices with Zstd compression, split into
  envelope and attachment indices. Batch-committed every 1,000 documents or
  60 seconds.
- **Fjall** — LZ4-compressed LSM tree key-value store, split into
  `email_keyspace` and `attachments_keyspace`. Content-hash addressed
  (BLAKE3), with insert-time deduplication.

### Content deduplication

Every ingested email is hashed with BLAKE3. Attachments are detached from the
MIME tree, hashed independently, and stored as raw undecoded bytes in Fjall's
`attachments_keyspace`. The email body is patched with hash-based placeholders
(`<<MAILBOXD_DETACH_HASH:...>>`) and stored in `email_keyspace`. Both keyspaces
check for existing hashes before writing — identical content is never stored
twice, regardless of which account or folder it arrives in.

## Storage & backup

### Data directory layout

```
{root}/
├── mailboxd-indices/   Tantivy full-text index (envelope + attachment)
├── mailboxd-storage/   Fjall LZ4-compressed blob store
├── memdb/              Metadata database (accounts, users, roles, config)
├── logs/               Server logs (when MAILBOXD_LOG_TO_FILE=true)
```

### Backup

Back up the entire `MAILBOXD_ROOT_DIR` (and `MAILBOXD_INDEX_DIR` /
`MAILBOXD_DATA_DIR` if overridden). **All three layers must be backed up
together** for consistency.

Do not place these directories on network-mounted storage.

### Encryption

Stored credentials (IMAP passwords, OAuth tokens) are encrypted with
AES-256-GCM via `ring`. The encryption key is derived from
`MAILBOXD_ENCRYPT_PASSWORD`.

## Contributing

Contributions are welcome — issues, merge requests, documentation.

Please open an issue on
<https://git.teccave.de/tecbeat/mailboxd/-/issues> before starting
non-trivial work.

### Commit messages

Format: `<type>(<scope>): <subject>` (imperative, present tense, no period).

Types: `fix`, `feat`, `refactor`, `ci`, `test`, `docs`, `chore`.

## Tech stack

| Layer | Technology |
|-------|-----------|
| Backend | Rust, Tokio, Poem + Poem OpenAPI |
| Full-text search | Tantivy (Zstd compression) |
| Blob storage | Fjall (LSM tree, LZ4 compression, KV separation) |
| Metadata DB | memdb (embedded key-value store with WAL) |
| IMAP | async-imap, rustls (ring), SOCKS5 proxy support |
| SMTP | embedded receiver (AUTH PLAIN/LOGIN, STARTTLS/TLS) |
| Cryptography | AES-256-GCM (ring), BLAKE3 (content hashing) |
| Frontend | React 18, TypeScript, Vite 6, ShadCN UI, TanStack Router/Query/Table |
| i18n | i18next (18 languages) |
| Container | Ubuntu 24.04, Docker |

## License and attribution

mailboxd is licensed under the
[GNU Affero General Public License v3.0 or later](LICENSE).

Copyright © 2026 tecbeat.
Copyright © 2025–2026 rustmailer.com.

mailboxd is a fork of the [Bichon email archiving project][bichon-upstream]
by rustmailer.com, distributed under the same AGPL-3.0-or-later license.
See [NOTICE](NOTICE) for the full attribution and a list of modifications.

In accordance with AGPL-3.0 § 13, the complete corresponding source code of
this modified version is available at
<https://git.teccave.de/tecbeat/mailboxd>. The web UI includes a link to the
source on the sign-in screen and in the dashboard footer.

[bichon-upstream]: https://github.com/rustmailer/bichon

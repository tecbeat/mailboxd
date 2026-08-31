<div align="center">

# Mailboxd

**Self-hosted email archiving in Rust — multi-account IMAP, full-text search, RBAC, OIDC, and an embedded WebUI.**

<p>
  <a href="https://git.teccave.de/tecbeat/mailboxd/-/commits/main"><img src="https://git.teccave.de/tecbeat/mailboxd/-/badges/main/pipeline.svg" alt="Pipeline Status"></a>
  <a href="https://git.teccave.de/tecbeat/mailboxd/-/releases"><img src="https://git.teccave.de/tecbeat/mailboxd/-/badges/release.svg" alt="Latest Release"></a>
  <a href="./LICENSE"><img src="https://img.shields.io/badge/license-AGPLv3-blue?style=flat-square" alt="License"></a>
</p>

<p>
  <a href="https://git.teccave.de/tecbeat/mailboxd/-/issues"><img src="https://img.shields.io/gitlab/issues/open/tecbeat%2Fmailboxd?gitlab_url=https%3A%2F%2Fgit.teccave.de&style=flat-square&label=Issues&color=orange" alt="Open Issues"></a>
  <a href="https://git.teccave.de/tecbeat/mailboxd/-/merge_requests"><img src="https://img.shields.io/gitlab/merge-requests/open/tecbeat%2Fmailboxd?gitlab_url=https%3A%2F%2Fgit.teccave.de&style=flat-square&label=Merge%20Requests&color=blue" alt="Open Merge Requests"></a>
  <a href="https://git.teccave.de/tecbeat/mailboxd/-/commits/main"><img src="https://img.shields.io/gitlab/last-commit/tecbeat%2Fmailboxd?gitlab_url=https%3A%2F%2Fgit.teccave.de&style=flat-square&label=Last%20commit" alt="Last commit"></a>
  <a href="https://git.teccave.de/tecbeat/mailboxd/-/wikis/home"><img src="https://img.shields.io/badge/Wiki-informational?style=flat-square" alt="Wiki"></a>
</p>

</div>

---

mailboxd is a self-hosted email archiving server built in Rust and a fork of Bichon by rustmailer.com. It concurrently downloads mail from any number of IMAP accounts (password or OAuth 2.0), deduplicates content via BLAKE3, indexes everything with Tantivy full-text search, and stores raw message content in an embedded, content-addressed blob store with Zstd compression. Access is via a documented REST API and an embedded React WebUI localised into 18 languages. Distributed under AGPL-3.0-or-later.

## Contents

- [Quick Start](#quick-start)
- [Prerequisites](#prerequisites)
- [Features](#features)
- [Configuration](#configuration)
- [Stack](#stack)
- [Development](#development)
- [FAQ](#faq)
- [Contributing](#contributing)
- [License](#license)

## Quick Start

```bash
docker compose up -d
```

## Prerequisites

- **Docker** — 24.0 or later with the Compose plugin, or a compatible container runtime.
- **Storage** — A local filesystem (ext4, XFS) for the data volume. NFS, CIFS and other network filesystems are NOT supported.
- **Ports** — One free TCP port for the WebUI/REST API (default 15630). Additional ports if the embedded SMTP (2525) or IMAP (10143/10993) servers are enabled.

## Features

- **Multi-account IMAP download** — Concurrent per-account sync with PLAIN/LOGIN and SASL XOAUTH2, automatic OAuth token refresh with PKCE, and support for SSL/TLS, STARTTLS or plain connections.
- **Incremental delta fetch** — UID-based delta fetching downloads only new messages after the initial sync. UIDVALIDITY changes trigger automatic cache rebuilds.
- **Fetch scoping** — Filter downloads by date range, mailbox folder limit or specific folder names. Optional per-account SOCKS5 proxy routing.
- **Auto-configuration** — Discover IMAP server settings automatically from an email domain.
- **Full-text search** — Tantivy indices for envelopes and attachments with Zstd compression. Optimised for European languages.
- **Advanced filters** — Date range, size range, attachment presence, file type, content category and facet-based tag combinations.
- **Thread grouping** — Reconstruct and view complete conversation threads across folders.
- **Attachment search** — Browse and filter attachments by sender, file type, size and other attachment properties.
- **Faceted tags** — Add, remove or overwrite tags on messages and attachments; filter by tag combinations with real-time count updates.
- **Contacts view** — Extracted and deduplicated sender/recipient address book across all authorised accounts.
- **Three-layer storage** — memdb for relational metadata, Tantivy for full-text indexing and mailboxd-blob (embedded, content-addressed, Zstd-compressed) for raw message and attachment storage — all embedded, no external dependencies.
- **Content deduplication** — BLAKE3 content hashing gives insert-time deduplication across accounts and folders. Folder moves update metadata only.
- **Dashboard analytics** — Email volume trends, top senders, storage usage breakdown, attachment statistics and per-account activity — scoped by user permissions.
- **OpenAPI 3.0** — All endpoints documented and browsable via Swagger UI, ReDoc and Scalar. Long-lived API tokens for programmatic access.
- **Multi-user RBAC** — 5 built-in roles (Admin, Manager, Member, AccountManager, AccountViewer) plus custom roles with 22 granular permissions. Account-level ACLs scope data access per user.
- **OpenID Connect SSO** — Authorization Code flow with PKCE against any OIDC provider (Authentik, Keycloak, PocketID, Authelia, Zitadel, Dex). Auto-provisioning of new users with a configurable default role.
- **Embedded SMTP receiver** — Optional built-in SMTP server for direct email ingestion. STARTTLS or implicit TLS, AUTH PLAIN/LOGIN with API-token authentication.
- **Embedded IMAP server** — Optional read-only IMAP server exposes archived mail to standard email clients (Thunderbird, Outlook, Apple Mail).
- **Import and export** — CLI tools import from EML directories, MBOX files (incl. Gmail's variant), Thunderbird profiles and Outlook PST files. Export as MBOX. All parsing happens server-side.
- **Scheduled downloads** — Per-account cron expressions run syncs at specific times — nightly-only or business-hours-only archiving.
- **Remote-content blocking** — External images and tracking pixels in emails are blocked by default; users can selectively allow remote content per message.
- **Async index dedup** — Duplicate detection in the search index runs asynchronously to reduce write latency during high-throughput ingestion.
- **Multi-language WebUI** — React + TypeScript frontend served embedded, localised into 18 languages with light and dark themes.

## Configuration

Every setting is available as a CLI flag and as the environment variable shown below. Environment variables can be supplied via the container runtime, a `.env` file, or the shell — CLI flags take precedence.

<details>
<summary><strong>Storage & Encryption</strong></summary>
<br/>

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `MAILBOXD_ROOT_DIR` | **Yes** | — | Absolute path for all persistent data. |
| `MAILBOXD_ENCRYPT_PASSWORD` | — | — | Password used to encrypt stored credentials (IMAP passwords, OAuth tokens). Alternative: `MAILBOXD_ENCRYPT_PASSWORD_FILE`. |
| `MAILBOXD_ENCRYPT_PASSWORD_FILE` | — | — | Read the encryption password from a file. If both are set, `MAILBOXD_ENCRYPT_PASSWORD` takes precedence. |
| `MAILBOXD_INDEX_DIR` | — | `{root}/mailboxd-indices` | Tantivy full-text index directory. Place on fast SSD. |
| `MAILBOXD_DATA_DIR` | — | `{root}/mailboxd-storage` | Blob storage directory. Can live on high-capacity HDD. |

</details>

<details>
<summary><strong>Server & Networking</strong></summary>
<br/>

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `MAILBOXD_HTTP_PORT` | — | `15630` | HTTP server port. |
| `MAILBOXD_BIND_IP` | — | `0.0.0.0` | IP address to bind to (IPv4 or IPv6). |
| `MAILBOXD_PUBLIC_URL` | — | `http://localhost:15630` | Public-facing URL used in OAuth redirects and docs. |
| `MAILBOXD_BASE_URL` | — | `/` | Base path for the WebUI when behind a reverse proxy (e.g. `/mailboxd`). |
| `MAILBOXD_ENABLE_REST_HTTPS` | — | `false` | Serve the REST API over HTTPS (requires a valid certificate). |
| `MAILBOXD_HTTP_COMPRESSION_ENABLED` | — | `true` | Enable gzip/brotli/zstd response compression. |
| `MAILBOXD_WEBUI_TOKEN_EXPIRATION_HOURS` | — | `168` | Access token lifetime in hours (default 7 days). |
| `MAILBOXD_SYNC_CONCURRENCY` | — | `num_cpus × 2` | Maximum concurrent account sync tasks. |

</details>

<details>
<summary><strong>Logging</strong></summary>
<br/>

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `MAILBOXD_LOG_LEVEL` | — | `info` | Log level: `trace`, `debug`, `info`, `warn`, `error`. |
| `MAILBOXD_ANSI_LOGS` | — | `true` | Colorised terminal output. |
| `MAILBOXD_JSON_LOGS` | — | `false` | JSON-formatted logs for log aggregators. |
| `MAILBOXD_LOG_TO_FILE` | — | `false` | Persist logs to files under the root dir. |
| `MAILBOXD_MAX_SERVER_LOG_FILES` | — | `5` | Maximum log files to retain. |

</details>

<details>
<summary><strong>CORS</strong></summary>
<br/>

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `MAILBOXD_CORS_ORIGINS` | — | — | Comma-separated list of allowed origins. If unset, all origins are allowed; if set, only exact matches pass (no wildcards, no trailing slash). |
| `MAILBOXD_CORS_MAX_AGE` | — | `86400` | CORS preflight cache duration in seconds. |

</details>

<details>
<summary><strong>Upload Limits</strong></summary>
<br/>

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `MAILBOXD_UPLOAD_BODY_LIMIT_MB` | — | `1100` | Maximum HTTP request body size in MB for file uploads. |
| `MAILBOXD_WEB_MBOX_UPLOAD_LIMIT_MB` | — | `1024` | Maximum per-file size in MB for MBOX uploads via the WebUI. |
| `MAILBOXD_WEB_PST_UPLOAD_LIMIT_MB` | — | `2048` | Maximum per-file size in MB for PST uploads via the WebUI. |

</details>

<details>
<summary><strong>Embedded SMTP Server</strong></summary>
<br/>

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `MAILBOXD_ENABLE_SMTP` | — | `false` | Enable the embedded SMTP receiver. |
| `MAILBOXD_SMTP_PORT` | — | `2525` | SMTP listening port (port 25 may require root). |
| `MAILBOXD_SMTP_ENCRYPTION` | — | `starttls` | SMTP encryption mode: `none`, `starttls`, `tls`. |
| `MAILBOXD_SMTP_AUTH_REQUIRED` | — | `true` | Require authentication for SMTP connections. |
| `MAILBOXD_TLS_KEY_PATH` | — | — | Absolute path to the SMTP/REST TLS private key. |
| `MAILBOXD_TLS_CERT_PATH` | — | — | Absolute path to the SMTP/REST TLS certificate chain. |

</details>

<details>
<summary><strong>Embedded IMAP Server</strong></summary>
<br/>

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `MAILBOXD_ENABLE_IMAP` | — | `false` | Enable the embedded read-only IMAP server. |
| `MAILBOXD_IMAP_PORT` | — | `10143` | IMAP port (STARTTLS or plaintext). |
| `MAILBOXD_IMAPS_PORT` | — | `10993` | IMAPS port (implicit TLS). |
| `MAILBOXD_IMAP_ENCRYPTION` | — | `none` | IMAP encryption mode: `none`, `starttls`, `tls`. |

</details>

<details>
<summary><strong>OIDC Single Sign-On</strong></summary>
<br/>

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `MAILBOXD_OIDC_ENABLED` | — | `false` | Enable OIDC single sign-on. When `true`, the four settings below are required. |
| `MAILBOXD_OIDC_ISSUER_URL` | — | — | Issuer URL of the OIDC provider (without the `/.well-known/openid-configuration` suffix). |
| `MAILBOXD_OIDC_CLIENT_ID` | — | — | OAuth 2.0 client ID registered with the IdP. |
| `MAILBOXD_OIDC_CLIENT_SECRET` | — | — | OAuth 2.0 client secret registered with the IdP. |
| `MAILBOXD_OIDC_REDIRECT_URI` | — | — | Redirect URI registered with the IdP. Must resolve to `<public-url>/api/auth/oidc/callback`. |
| `MAILBOXD_OIDC_DEFAULT_ROLE_ID` | — | `100200000000000` | Global role ID assigned to auto-provisioned OIDC users (built-in `Member`). |
| `MAILBOXD_OIDC_AUTO_REDIRECT` | — | `false` | When OIDC is configured, `/sign-in` redirects to the IdP immediately. The local login form remains reachable via `/sign-in?local=1`. |

</details>


## Stack

**Backend:** Rust, Tokio, Poem + Poem OpenAPI. **Full-text search:** Tantivy (Zstd compression). **Blob storage:** mailboxd-blob (embedded content-addressed store, Zstd compression, BLAKE3 dedup). **Metadata DB:** memdb (embedded key-value store with WAL). **IMAP:** async-imap, rustls (ring), SOCKS5 proxy support. **SMTP/IMAP servers:** embedded (AUTH PLAIN/LOGIN, STARTTLS/TLS). **Cryptography:** AES-256-GCM (ring), BLAKE3 (content hashing). **Frontend:** React 18, TypeScript, Vite, ShadCN UI, TanStack Router/Query/Table. **i18n:** i18next (18 languages). **Container:** Ubuntu 24.04, Docker.

## Development

Prerequisites: Rust (latest stable), Node.js 20+, pnpm. Clone the repo and run `cargo build` — the frontend is installed and built automatically via `build.rs`. For frontend development run `cd web && pnpm run dev` for a Vite dev server with API proxy to the Rust backend. See [CONTRIBUTING guidelines](https://git.teccave.de/tecbeat/mailboxd/-/blob/main/README.md#contributing).

## FAQ

<details>
<summary><strong>Upgrading from Bichon 2.x</strong></summary>

mailboxd is a drop-in successor to Bichon 2.x and reuses the same on-disk format (storage version 2). To migrate: stop Bichon, swap the container image for mailboxd, and rename every `BICHON_*` environment variable to `MAILBOXD_*` (e.g. `BICHON_ROOT_DIR` → `MAILBOXD_ROOT_DIR`), keeping the **same** `MAILBOXD_ENCRYPT_PASSWORD` as your previous `BICHON_ENCRYPT_PASSWORD` so stored IMAP and OAuth credentials remain decryptable. Point `MAILBOXD_ROOT_DIR` at your existing Bichon data volume and start the container — on first boot mailboxd renames the `bichon-indices` and `bichon-storage` directories to `mailboxd-indices` and `mailboxd-storage` and adopts the database in place, with no re-download and no re-index. Existing users, roles, accounts and archived mail are preserved; sign in with your previous credentials. Back up the data volume before upgrading.

</details>

<details>
<summary><strong>Upgrading from an older data layout (storage v1 or v0.3.7)</strong></summary>

Older installs used earlier on-disk layouts — the fjall-based v1.x blob storage (Bichon 1.x era) or the Tantivy-based v0.3.7 layout. mailboxd migrates these to the current v2 layout automatically: the server runs the migration on startup, before it begins serving, with no operator interaction. The migration is non-destructive and idempotent — it reads the legacy data, writes the v2 store alongside it and records a `STORAGE_VERSION` marker — so it is a no-op on volumes already on v2 and safe to run on every start. Just swap the image for the new version and start the container. If migration fails the server refuses to start rather than run against half-converted data; check the startup logs. A partially-completed v0.3.7 migration is not resumed automatically — run the interactive `mailboxd-admin` tool in that case. **Back up the data volume before upgrading.**

</details>

<details>
<summary><strong>CORS errors when accessing the WebUI</strong></summary>

Enable debug logging with `MAILBOXD_LOG_LEVEL=debug`, check the server logs for the incoming `Origin` header, and ensure the browser's exact origin matches an entry in `MAILBOXD_CORS_ORIGINS` (no trailing slash, no wildcards). In Docker, do not wrap the value in quotes.

</details>

<details>
<summary><strong>How do I run mailboxd behind a reverse proxy?</strong></summary>

Set `MAILBOXD_BASE_URL=/mailboxd` (or your sub-path) and configure your proxy to forward `/mailboxd/` to `http://127.0.0.1:15630/`, preserving `Host` and `X-Forwarded-For` headers.

</details>

<details>
<summary><strong>Can mailboxd send emails?</strong></summary>

No. mailboxd is an **archiver**, not an email client. The optional SMTP server only **receives** emails — it cannot send, forward or reply. The optional IMAP server is read-only.

</details>

<details>
<summary><strong>What hardware does mailboxd need?</strong></summary>

Recommended: 4+ CPU cores, 2+ GB RAM (sufficient for 10+ accounts and 200+ GB of archived data). Use a mainstream local filesystem (ext4, XFS); avoid network/virtual filesystems (NFS, SMB, VirtIO-FS) for all data directories. Indices benefit from SSD storage; blob storage can use HDD.

</details>

<details>
<summary><strong>How do I reset the admin password?</strong></summary>

Run the `mailboxd-admin` binary inside the container (`docker exec -it mailboxd mailboxd-admin`) and select *Reset Admin Password*. `MAILBOXD_ENCRYPT_PASSWORD` (or its `_FILE` counterpart) must be set for the tool to decrypt stored credentials.

</details>

<details>
<summary><strong>How do I back up my data?</strong></summary>

Back up the entire `MAILBOXD_ROOT_DIR` (plus `MAILBOXD_INDEX_DIR` and `MAILBOXD_DATA_DIR` if overridden). All three storage layers must be backed up together for consistency. Do not place any of these directories on network-mounted storage — sync locally, then rsync to a remote destination.

</details>

<details>
<summary><strong>Default login credentials?</strong></summary>

On first start, mailboxd creates a built-in admin user with username `admin` and password `admin@mailboxd`. **Change the password immediately** via *Settings → Profile* in the WebUI.

</details>


## Contributing

Contributions are welcome. Please open an [Issue](https://git.teccave.de/tecbeat/mailboxd/-/issues) or [Merge Request](https://git.teccave.de/tecbeat/mailboxd/-/merge_requests) on GitLab.

## License

Mailboxd is Free Software: You can use, study, share, and improve it at your will. Specifically you can redistribute and/or modify it under the terms of the [AGPLv3 License](./LICENSE).

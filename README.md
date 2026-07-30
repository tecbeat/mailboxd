<div align="center">

# Mailboxd

[![Pipeline Status](https://git.teccave.de/tecbeat/mailboxd/badges/main/pipeline.svg)](https://git.teccave.de/tecbeat/mailboxd/-/commits/main)
[![Latest Release](https://git.teccave.de/tecbeat/mailboxd/-/badges/release.svg)](https://git.teccave.de/tecbeat/mailboxd/-/releases)
[![License](https://img.shields.io/badge/license-GPLv3-blue)](./LICENSE)

</div>

---

mailboxd is a self-hosted email archiving server built in Rust. It downloads mail from IMAP accounts via password or OAuth2, deduplicates content with BLAKE3, indexes everything with Tantivy full-text search, and stores the raw blobs in an LZ4-compressed LSM tree. Access is via a REST API and an embedded React web UI. mailboxd is a fork of Bichon by rustmailer.com and is distributed under AGPL-3.0-or-later.

### Quick Start

```bash
docker compose up -d
```

### Features

* **Multi-account IMAP download**: Concurrent IMAP sync per account with PLAIN/LOGIN and SASL XOAUTH2, incremental UID-based delta fetching, UIDVALIDITY-aware cache rebuilds, per-account SOCKS5 proxy and cron-based schedules.

* **Full-text search**: Tantivy indices for envelopes and attachments with Zstd compression, faceted tags, thread reconstruction, and filters for date range, size, attachment presence, file type and content category.

* **Three-layer storage**: memdb for relational metadata, Tantivy for full-text indexing, and Fjall for LZ4-compressed content-addressed blob storage. BLAKE3 content hashing gives insert-time deduplication across accounts and folders.

* **Multi-user RBAC**: 5 built-in roles (Admin, Manager, Member, AccountManager, AccountViewer) plus custom roles with 22 granular permissions. Account-level ACLs scope data access per user.

* **OpenID Connect SSO**: Authorization Code flow with PKCE against any OIDC provider (Authentik, Keycloak, PocketID, Authelia, Zitadel, Dex). Auto-provisioning of new users with a configurable default role.

* **Embedded SMTP receiver**: Optional built-in SMTP server for direct email ingestion. STARTTLS or implicit TLS, AUTH PLAIN/LOGIN with API-token authentication.

* **Import and export**: CLI tools import from EML directories, MBOX files (incl. Gmail's variant), Thunderbird profiles and Outlook PST files. Export as MBOX. All parsing happens server-side.

* **REST API with OpenAPI 3.0**: All endpoints documented and browsable via Swagger UI, ReDoc and Scalar. Long-lived API tokens for programmatic access.

* **Multi-language web UI**: React + TypeScript frontend served embedded, localised into 18 languages with light and dark themes.


### Contributing

Contributions are welcome. Please open an [Issue](https://git.teccave.de/tecbeat/mailboxd/-/issues) or [Merge Request](https://git.teccave.de/tecbeat/mailboxd/-/merge_requests) on GitLab.

### License

Mailboxd is Free Software: You can use, study, share, and improve it at your will. Specifically you can redistribute and/or modify it under the terms of the [GPLv3 License](./LICENSE).

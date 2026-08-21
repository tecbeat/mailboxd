#!/bin/sh
set -e

# Bring an existing data volume up to the v2 mailboxd-blob layout before the
# server starts:
#   * adopt a Bichon volume in place (rename bichon-* → mailboxd-*),
#   * convert a v1.x fjall blob store to mailboxd-blob, or
#   * convert a legacy v0.3.7 Tantivy layout to v2.
# Idempotent and non-destructive: a no-op on fresh installs and on volumes
# already on v2, and it never deletes the legacy source data.
/usr/local/bin/mailboxd-admin --auto-migrate

exec /opt/mailboxd/mailboxd-server "$@"

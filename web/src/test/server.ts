import { http, HttpResponse } from 'msw';
import { setupServer } from 'msw/node';

import type { ServerConfigurations } from '@/api/system/api';

// In tests, axiosInstance targets the dev backend base URL (see axiosInstance.ts).
const BASE = 'http://localhost:15630';

/** A complete ServerConfigurations with sane defaults; override only what a test cares about. */
export function makeSysConfig(overrides: Partial<ServerConfigurations> = {}): ServerConfigurations {
  return {
    mailboxd_log_level: 'info',
    mailboxd_http_port: 15630,
    mailboxd_bind_ip: null,
    mailboxd_base_url: '',
    mailboxd_public_url: '',
    mailboxd_enable_rest_https: false,
    mailboxd_http_compression_enabled: false,
    mailboxd_cors_origins: null,
    mailboxd_cors_max_age: 0,
    mailboxd_ansi_logs: false,
    mailboxd_log_to_file: false,
    mailboxd_json_logs: false,
    mailboxd_max_server_log_files: 0,
    mailboxd_encrypt_password_set: false,
    mailboxd_webui_token_expiration_hours: 0,
    mailboxd_root_dir: '/',
    mailboxd_index_dir: null,
    mailboxd_data_dir: null,
    mailboxd_metadata_cache_size: 0,
    mailboxd_envelope_cache_size: 0,
    mailboxd_sync_concurrency: null,
    mailboxd_enable_smtp: false,
    mailboxd_smtp_port: 0,
    mailboxd_smtp_encryption: 'none',
    mailboxd_smtp_auth_required: false,
    mailboxd_smtp_tls_key_path: null,
    mailboxd_smtp_tls_cert_path: null,
    mailboxd_upload_body_limit_mb: 0,
    mailboxd_web_mbox_upload_limit_mb: 1024,
    mailboxd_web_pst_upload_limit_mb: 2048,
    ...overrides,
  };
}

// Benign defaults so components render without hitting the network.
// Individual tests override with `server.use(...)`.
export const handlers = [
  http.get(`${BASE}/api/v1/accounts`, () => HttpResponse.json({ items: [] })),
  http.get(`${BASE}/api/v1/import-history`, () => HttpResponse.json([])),
  http.get(`${BASE}/api/v1/notifications`, () =>
    HttpResponse.json({ release: { latest: null, is_newer: false, error_message: null } }),
  ),
  http.get(`${BASE}/api/v1/system-configurations`, () => HttpResponse.json(makeSysConfig())),
];

export const server = setupServer(...handlers);

//
// Copyright (c) 2026 tecbeat
//
// This file is part of mailboxd, an email archiving project.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use crate::error::code::ErrorCode;
use crate::error::MailboxdResult;
use crate::oidc::id_token::{is_symmetric, select_jwk_key};
use crate::raise_error;
use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{DecodingKey, Header};
use std::future::Future;
use std::sync::RwLock;
use std::time::{Duration, Instant};

const JWKS_CACHE_TTL: Duration = Duration::from_secs(3600);
const JWKS_HTTP_TIMEOUT: Duration = Duration::from_secs(15);

struct CachedJwks {
    jwks: JwkSet,
    fetched_at: Instant,
    jwks_uri: String,
}

static JWKS_CACHE: RwLock<Option<CachedJwks>> = RwLock::new(None);

/// Return the provider's JWKS, from cache unless `force_refresh` is set or
/// the cached copy is stale or belongs to another `jwks_uri`.
pub async fn get_jwks(jwks_uri: &str, force_refresh: bool) -> MailboxdResult<JwkSet> {
    if !force_refresh {
        if let Ok(guard) = JWKS_CACHE.read() {
            if let Some(cached) = guard.as_ref() {
                if cached.jwks_uri == jwks_uri && cached.fetched_at.elapsed() < JWKS_CACHE_TTL {
                    return Ok(cached.jwks.clone());
                }
            }
        }
    }

    let jwks = fetch_jwks(jwks_uri).await?;

    if let Ok(mut guard) = JWKS_CACHE.write() {
        *guard = Some(CachedJwks {
            jwks: jwks.clone(),
            fetched_at: Instant::now(),
            jwks_uri: jwks_uri.to_string(),
        });
    }

    Ok(jwks)
}

async fn fetch_jwks(jwks_uri: &str) -> MailboxdResult<JwkSet> {
    let client = reqwest::Client::builder()
        .timeout(JWKS_HTTP_TIMEOUT)
        .build()
        .map_err(|e| {
            raise_error!(
                format!("Failed to build HTTP client for OIDC JWKS: {}", e),
                ErrorCode::InternalError
            )
        })?;

    let resp = client.get(jwks_uri).send().await.map_err(|e| {
        raise_error!(
            format!("OIDC JWKS request to {} failed: {}", jwks_uri, e),
            ErrorCode::HttpResponseError
        )
    })?;

    if !resp.status().is_success() {
        return Err(raise_error!(
            format!(
                "OIDC JWKS endpoint returned non-success status {} for {}",
                resp.status(),
                jwks_uri
            ),
            ErrorCode::HttpResponseError
        ));
    }

    resp.json::<JwkSet>().await.map_err(|e| {
        raise_error!(
            format!("Failed to parse OIDC JWKS document: {}", e),
            ErrorCode::HttpResponseError
        )
    })
}

/// Resolve the key that verifies a token with `header`.
///
/// HMAC algorithms use the OAuth client secret. Asymmetric algorithms use
/// the provider's JWKS via `load(force_refresh)`: the cached set first, and
/// exactly one forced refetch when no key matches (the provider rotated its
/// signing key). If the refreshed set still has no match, verification fails.
pub async fn resolve_decoding_key<F, Fut>(
    header: &Header,
    client_secret: &[u8],
    load: F,
) -> MailboxdResult<DecodingKey>
where
    F: Fn(bool) -> Fut,
    Fut: Future<Output = MailboxdResult<JwkSet>>,
{
    if is_symmetric(header.alg) {
        return Ok(DecodingKey::from_secret(client_secret));
    }

    if let Some(key) = select_jwk_key(header, &load(false).await?)? {
        return Ok(key);
    }
    select_jwk_key(header, &load(true).await?)?.ok_or_else(|| {
        raise_error!(
            format!(
                "No key in the provider's JWKS matches the ID token (kid: {})",
                header.kid.as_deref().unwrap_or("<none>")
            ),
            ErrorCode::PermissionDenied
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oidc::id_token::test_keys::TestKey;
    use jsonwebtoken::Algorithm;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    fn header(alg: Algorithm, kid: Option<&str>) -> Header {
        let mut h = Header::new(alg);
        h.kid = kid.map(str::to_string);
        h
    }

    #[tokio::test]
    async fn hmac_uses_client_secret_without_loading_jwks() {
        let calls = AtomicUsize::new(0);
        let key = resolve_decoding_key(&header(Algorithm::HS256, None), b"secret", |_| {
            calls.fetch_add(1, Ordering::SeqCst);
            async { Ok(JwkSet { keys: vec![] }) }
        })
        .await;
        assert!(key.is_ok());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn cached_jwks_with_matching_kid_needs_no_refresh() {
        let key = TestKey::es256();
        let jwk = key.public_jwk(Some("k1"));
        let refreshes = Mutex::new(Vec::new());
        let resolved = resolve_decoding_key(&header(Algorithm::ES256, Some("k1")), b"", |force| {
            refreshes.lock().unwrap().push(force);
            let set = JwkSet {
                keys: vec![jwk.clone()],
            };
            async move { Ok(set) }
        })
        .await;
        assert!(resolved.is_ok());
        assert_eq!(*refreshes.lock().unwrap(), vec![false]);
    }

    #[tokio::test]
    async fn unknown_kid_refetches_once_and_picks_rotated_key() {
        let old = TestKey::es256().public_jwk(Some("old"));
        let new = TestKey::es256().public_jwk(Some("new"));
        let refreshes = Mutex::new(Vec::new());
        let resolved = resolve_decoding_key(&header(Algorithm::ES256, Some("new")), b"", |force| {
            refreshes.lock().unwrap().push(force);
            let set = JwkSet {
                keys: vec![if force { new.clone() } else { old.clone() }],
            };
            async move { Ok(set) }
        })
        .await;
        assert!(resolved.is_ok());
        assert_eq!(*refreshes.lock().unwrap(), vec![false, true]);
    }

    #[tokio::test]
    async fn unknown_kid_after_refresh_fails_closed() {
        let only = TestKey::es256().public_jwk(Some("other"));
        let refreshes = Mutex::new(Vec::new());
        let resolved =
            resolve_decoding_key(&header(Algorithm::ES256, Some("missing")), b"", |force| {
                refreshes.lock().unwrap().push(force);
                let set = JwkSet {
                    keys: vec![only.clone()],
                };
                async move { Ok(set) }
            })
            .await;
        assert!(resolved.is_err());
        assert_eq!(*refreshes.lock().unwrap(), vec![false, true]);
    }

    #[tokio::test]
    async fn jwks_load_error_is_propagated() {
        let resolved = resolve_decoding_key(&header(Algorithm::RS256, Some("k")), b"", |_| async {
            Err(raise_error!("boom".into(), ErrorCode::HttpResponseError))
        })
        .await;
        assert!(resolved.is_err());
    }
}

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
use crate::raise_error;
use jsonwebtoken::jwk::{AlgorithmParameters, Jwk, JwkSet, KeyAlgorithm, PublicKeyUse};
use jsonwebtoken::{Algorithm, DecodingKey, Header, Validation};
use serde::Deserialize;
use serde_json::Value;

/// Constant-time byte-slice equality. Used for comparing security-sensitive
/// values (nonces, tokens) where a variable-time compare would leak the
/// value one byte at a time through timing side-channels.
fn eq_ct(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Claims we care about for user identification. Extra claims are ignored.
#[derive(Debug, Clone, Deserialize)]
pub struct IdTokenClaims {
    pub iss: String,
    #[serde(default)]
    pub aud: Value,
    pub sub: String,
    pub exp: i64,
    #[serde(default)]
    pub iat: Option<i64>,
    #[serde(default)]
    pub nonce: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub email_verified: Option<bool>,
    #[serde(default)]
    pub preferred_username: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub given_name: Option<String>,
}

pub struct VerifyParams<'a> {
    pub expected_issuer: &'a str,
    pub expected_audience: &'a str,
    pub expected_nonce: &'a str,
    /// Clock skew tolerance in seconds.
    pub clock_skew_secs: i64,
    /// Current unix time in seconds.
    pub now_secs: i64,
}

/// Parse the (unverified) JOSE header to learn `alg` and `kid`, which decide
/// how the verification key is obtained. Unknown algorithms, including
/// `none`, fail here.
pub fn decode_header(token: &str) -> MailboxdResult<Header> {
    jsonwebtoken::decode_header(token).map_err(|e| {
        raise_error!(
            format!("Unsupported or malformed ID token header: {}", e),
            ErrorCode::PermissionDenied
        )
    })
}

/// Reject an algorithm the provider does not advertise in its discovery
/// document. An empty list (field absent) imposes no restriction.
pub fn ensure_alg_advertised(alg: Algorithm, advertised: &[String]) -> MailboxdResult<()> {
    if advertised.is_empty() || advertised.iter().any(|a| *a == alg_name(alg)) {
        return Ok(());
    }
    Err(raise_error!(
        format!(
            "ID token signing algorithm '{}' is not advertised by the provider",
            alg_name(alg)
        ),
        ErrorCode::PermissionDenied
    ))
}

fn alg_name(alg: Algorithm) -> String {
    KeyAlgorithm::from(alg).to_string()
}

pub fn is_symmetric(alg: Algorithm) -> bool {
    matches!(alg, Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512)
}

fn jwk_usable_for(jwk: &Jwk, alg: Algorithm) -> bool {
    if matches!(jwk.common.public_key_use, Some(PublicKeyUse::Encryption)) {
        return false;
    }
    if let Some(key_alg) = jwk.common.key_algorithm {
        if key_alg != KeyAlgorithm::from(alg) {
            return false;
        }
    }
    match &jwk.algorithm {
        AlgorithmParameters::RSA(_) => matches!(
            alg,
            Algorithm::RS256
                | Algorithm::RS384
                | Algorithm::RS512
                | Algorithm::PS256
                | Algorithm::PS384
                | Algorithm::PS512
        ),
        AlgorithmParameters::EllipticCurve(_) => {
            matches!(alg, Algorithm::ES256 | Algorithm::ES384)
        }
        AlgorithmParameters::OctetKeyPair(_) => alg == Algorithm::EdDSA,
        // Symmetric keys are never accepted from a JWKS; HMAC uses the client secret.
        _ => false,
    }
}

/// Pick the JWKS key that verifies a token with the given header.
///
/// With a `kid` the key must carry that id; without one the set must contain
/// exactly one usable key for the algorithm. `Ok(None)` means no matching key
/// was found, so the caller may refetch the JWKS once (key rotation).
pub fn select_jwk_key(header: &Header, jwks: &JwkSet) -> MailboxdResult<Option<DecodingKey>> {
    let mut candidates = jwks.keys.iter().filter(|jwk| {
        let kid_matches = match header.kid.as_deref() {
            Some(kid) => jwk.common.key_id.as_deref() == Some(kid),
            None => true,
        };
        kid_matches && jwk_usable_for(jwk, header.alg)
    });

    let Some(jwk) = candidates.next() else {
        return Ok(None);
    };
    if header.kid.is_none() && candidates.next().is_some() {
        return Err(raise_error!(
            "ID token has no 'kid' and the provider's JWKS contains several matching keys".into(),
            ErrorCode::PermissionDenied
        ));
    }

    DecodingKey::from_jwk(jwk).map(Some).map_err(|e| {
        raise_error!(
            format!("Invalid key in provider JWKS: {}", e),
            ErrorCode::PermissionDenied
        )
    })
}

fn audience_matches(claim: &Value, expected: &str) -> bool {
    match claim {
        Value::String(s) => s == expected,
        Value::Array(arr) => arr.iter().any(|v| v.as_str() == Some(expected)),
        _ => false,
    }
}

/// Verify the signature of `token` with `key` and validate the standard OIDC
/// claims: `iss`, `aud`, `exp` (with configurable skew) and `nonce`
/// (constant-time compare).
///
/// Only the algorithm from the token header is accepted, and the key family
/// must match it — so an HMAC token can never be checked against a public key.
pub fn verify_and_parse(
    token: &str,
    key: &DecodingKey,
    params: &VerifyParams<'_>,
) -> MailboxdResult<IdTokenClaims> {
    let header = decode_header(token)?;

    let mut validation = Validation::new(header.alg);
    validation.required_spec_claims.clear();
    validation.validate_exp = false;
    validation.validate_nbf = false;
    validation.validate_aud = false;

    let claims = jsonwebtoken::decode::<IdTokenClaims>(token, key, &validation)
        .map_err(|e| {
            raise_error!(
                format!("ID token signature verification failed: {}", e),
                ErrorCode::PermissionDenied
            )
        })?
        .claims;

    if claims.iss.trim_end_matches('/') != params.expected_issuer.trim_end_matches('/') {
        return Err(raise_error!(
            format!(
                "ID token issuer mismatch: expected '{}', got '{}'",
                params.expected_issuer, claims.iss
            ),
            ErrorCode::PermissionDenied
        ));
    }

    if !audience_matches(&claims.aud, params.expected_audience) {
        return Err(raise_error!(
            "ID token audience does not include this client".into(),
            ErrorCode::PermissionDenied
        ));
    }

    if params.now_secs > claims.exp + params.clock_skew_secs {
        return Err(raise_error!(
            "ID token has expired".into(),
            ErrorCode::PermissionDenied
        ));
    }

    let nonce_ok = claims
        .nonce
        .as_deref()
        .map(|n| eq_ct(n.as_bytes(), params.expected_nonce.as_bytes()))
        .unwrap_or(false);
    if !nonce_ok {
        return Err(raise_error!(
            "ID token nonce mismatch — possible replay attack".into(),
            ErrorCode::PermissionDenied
        ));
    }

    Ok(claims)
}

#[cfg(test)]
pub(crate) mod test_keys {
    use aws_lc_rs::encoding::AsDer;
    use aws_lc_rs::rand::SystemRandom;
    use aws_lc_rs::rsa::{KeyPair as RsaKeyPair, KeySize};
    use aws_lc_rs::signature::{EcdsaKeyPair, Ed25519KeyPair, ECDSA_P256_SHA256_FIXED_SIGNING};
    use base64::Engine as _;
    use jsonwebtoken::jwk::Jwk;
    use jsonwebtoken::{Algorithm, EncodingKey};

    pub struct TestKey {
        pub alg: Algorithm,
        pub encoding: EncodingKey,
    }

    impl TestKey {
        pub fn rsa(alg: Algorithm) -> Self {
            let pair = RsaKeyPair::generate(KeySize::Rsa2048).expect("generate RSA key");
            let der = AsDer::<aws_lc_rs::encoding::Pkcs8V1Der>::as_der(&pair).expect("RSA DER");
            let b64 = base64::engine::general_purpose::STANDARD.encode(der.as_ref());
            let body: Vec<String> = b64
                .as_bytes()
                .chunks(64)
                .map(|c| String::from_utf8_lossy(c).into_owned())
                .collect();
            let pem = format!(
                "-----BEGIN PRIVATE KEY-----\n{}\n-----END PRIVATE KEY-----\n",
                body.join("\n")
            );
            let encoding = EncodingKey::from_rsa_pem(pem.as_bytes()).expect("RSA PEM");
            Self { alg, encoding }
        }

        pub fn es256() -> Self {
            let pkcs8 = EcdsaKeyPair::generate_pkcs8(
                &ECDSA_P256_SHA256_FIXED_SIGNING,
                &SystemRandom::new(),
            )
            .expect("generate EC key");
            Self {
                alg: Algorithm::ES256,
                encoding: EncodingKey::from_ec_der(pkcs8.as_ref()),
            }
        }

        pub fn ed25519() -> Self {
            // jsonwebtoken derives the public JWK only from PKCS#8 v1 documents.
            let pkcs8 = Ed25519KeyPair::generate()
                .and_then(|pair| pair.to_pkcs8v1())
                .expect("generate Ed key");
            Self {
                alg: Algorithm::EdDSA,
                encoding: EncodingKey::from_ed_der(pkcs8.as_ref()),
            }
        }

        pub fn public_jwk(&self, kid: Option<&str>) -> Jwk {
            let mut jwk = Jwk::from_encoding_key(&self.encoding, self.alg).expect("public JWK");
            jwk.common.key_id = kid.map(str::to_string);
            jwk
        }

        pub fn sign(&self, kid: Option<&str>, claims: &serde_json::Value) -> String {
            let mut header = jsonwebtoken::Header::new(self.alg);
            header.kid = kid.map(str::to_string);
            jsonwebtoken::encode(&header, claims, &self.encoding).expect("sign token")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_keys::TestKey;
    use super::*;
    use jsonwebtoken::EncodingKey;
    use serde_json::json;

    const ISSUER: &str = "https://idp.example.test/application/o/mailboxd/";
    const CLIENT_ID: &str = "mailboxd";
    const NONCE: &str = "expected-nonce";
    const SECRET: &[u8] = b"test-client-secret-with-enough-bytes";
    const NOW: i64 = 1_800_000_000;

    fn claims() -> Value {
        json!({
            "iss": ISSUER,
            "aud": CLIENT_ID,
            "sub": "user-1",
            "exp": NOW + 300,
            "nonce": NONCE,
        })
    }

    fn params() -> VerifyParams<'static> {
        VerifyParams {
            expected_issuer: ISSUER,
            expected_audience: CLIENT_ID,
            expected_nonce: NONCE,
            clock_skew_secs: 60,
            now_secs: NOW,
        }
    }

    fn jwks(keys: Vec<Jwk>) -> JwkSet {
        JwkSet { keys }
    }

    fn verify_with_jwks(token: &str, set: &JwkSet) -> MailboxdResult<IdTokenClaims> {
        let header = decode_header(token)?;
        let key = select_jwk_key(&header, set)?.expect("matching key");
        verify_and_parse(token, &key, &params())
    }

    fn hs256_token(secret: &[u8], claims: &Value) -> String {
        jsonwebtoken::encode(
            &Header::new(Algorithm::HS256),
            claims,
            &EncodingKey::from_secret(secret),
        )
        .expect("sign HS256")
    }

    #[test]
    fn rs256_token_verifies_against_jwks() {
        let key = TestKey::rsa(Algorithm::RS256);
        let token = key.sign(Some("rsa-1"), &claims());
        let parsed = verify_with_jwks(&token, &jwks(vec![key.public_jwk(Some("rsa-1"))]))
            .expect("valid RS256 token");
        assert_eq!(parsed.sub, "user-1");
    }

    #[test]
    fn ps256_token_verifies_against_jwks() {
        let key = TestKey::rsa(Algorithm::PS256);
        let token = key.sign(Some("rsa-pss"), &claims());
        verify_with_jwks(&token, &jwks(vec![key.public_jwk(Some("rsa-pss"))]))
            .expect("valid PS256 token");
    }

    #[test]
    fn es256_token_verifies_against_jwks() {
        let key = TestKey::es256();
        let token = key.sign(Some("ec-1"), &claims());
        verify_with_jwks(&token, &jwks(vec![key.public_jwk(Some("ec-1"))]))
            .expect("valid ES256 token");
    }

    #[test]
    fn eddsa_token_verifies_against_jwks() {
        let key = TestKey::ed25519();
        let token = key.sign(Some("ed-1"), &claims());
        verify_with_jwks(&token, &jwks(vec![key.public_jwk(Some("ed-1"))]))
            .expect("valid EdDSA token");
    }

    #[test]
    fn token_without_kid_uses_the_single_matching_key() {
        let key = TestKey::es256();
        let token = key.sign(None, &claims());
        verify_with_jwks(&token, &jwks(vec![key.public_jwk(None)])).expect("valid token");
    }

    #[test]
    fn token_without_kid_and_ambiguous_jwks_is_rejected() {
        let a = TestKey::es256();
        let b = TestKey::es256();
        let token = a.sign(None, &claims());
        let header = decode_header(&token).unwrap();
        let set = jwks(vec![a.public_jwk(None), b.public_jwk(None)]);
        assert!(select_jwk_key(&header, &set).is_err());
    }

    #[test]
    fn unknown_kid_yields_no_key() {
        let key = TestKey::es256();
        let token = key.sign(Some("rotated"), &claims());
        let header = decode_header(&token).unwrap();
        let set = jwks(vec![key.public_jwk(Some("old"))]);
        assert!(select_jwk_key(&header, &set).unwrap().is_none());
    }

    #[test]
    fn token_signed_by_other_key_with_same_kid_is_rejected() {
        let signer = TestKey::rsa(Algorithm::RS256);
        let published = TestKey::rsa(Algorithm::RS256);
        let token = signer.sign(Some("k"), &claims());
        let err = verify_with_jwks(&token, &jwks(vec![published.public_jwk(Some("k"))]));
        assert!(err.is_err());
    }

    #[test]
    fn jwk_for_other_family_is_not_selected() {
        let rsa = TestKey::rsa(Algorithm::RS256);
        let ec = TestKey::es256();
        let token = ec.sign(Some("k"), &claims());
        let header = decode_header(&token).unwrap();
        let set = jwks(vec![rsa.public_jwk(Some("k"))]);
        assert!(select_jwk_key(&header, &set).unwrap().is_none());
    }

    #[test]
    fn symmetric_jwk_is_never_selected() {
        let token = hs256_token(SECRET, &claims());
        let header = decode_header(&token).unwrap();
        let oct =
            Jwk::from_encoding_key(&EncodingKey::from_secret(SECRET), Algorithm::HS256).unwrap();
        assert!(select_jwk_key(&header, &jwks(vec![oct])).unwrap().is_none());
    }

    #[test]
    fn hs256_token_verifies_with_client_secret() {
        let token = hs256_token(SECRET, &claims());
        verify_and_parse(&token, &DecodingKey::from_secret(SECRET), &params())
            .expect("valid HS256 token");
    }

    #[test]
    fn hs256_token_with_wrong_secret_is_rejected() {
        let token = hs256_token(b"some-other-secret", &claims());
        assert!(verify_and_parse(&token, &DecodingKey::from_secret(SECRET), &params()).is_err());
    }

    #[test]
    fn hs256_token_is_rejected_by_public_key() {
        let rsa = TestKey::rsa(Algorithm::RS256);
        let token = hs256_token(SECRET, &claims());
        let public = DecodingKey::from_jwk(&rsa.public_jwk(None)).unwrap();
        assert!(verify_and_parse(&token, &public, &params()).is_err());
    }

    #[test]
    fn alg_none_is_rejected() {
        let header = base64::Engine::encode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            br#"{"alg":"none","typ":"JWT"}"#,
        );
        let payload = base64::Engine::encode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            claims().to_string(),
        );
        assert!(decode_header(&format!("{header}.{payload}.")).is_err());
    }

    #[test]
    fn expired_token_is_rejected() {
        let key = TestKey::es256();
        let mut c = claims();
        c["exp"] = json!(NOW - 61);
        let token = key.sign(Some("k"), &c);
        assert!(verify_with_jwks(&token, &jwks(vec![key.public_jwk(Some("k"))])).is_err());
    }

    #[test]
    fn token_within_clock_skew_is_accepted() {
        let key = TestKey::es256();
        let mut c = claims();
        c["exp"] = json!(NOW - 30);
        let token = key.sign(Some("k"), &c);
        verify_with_jwks(&token, &jwks(vec![key.public_jwk(Some("k"))])).expect("within skew");
    }

    #[test]
    fn wrong_issuer_is_rejected() {
        let key = TestKey::es256();
        let mut c = claims();
        c["iss"] = json!("https://evil.example.test/");
        let token = key.sign(Some("k"), &c);
        assert!(verify_with_jwks(&token, &jwks(vec![key.public_jwk(Some("k"))])).is_err());
    }

    #[test]
    fn issuer_trailing_slash_is_normalised() {
        let key = TestKey::es256();
        let mut c = claims();
        c["iss"] = json!(ISSUER.trim_end_matches('/'));
        let token = key.sign(Some("k"), &c);
        verify_with_jwks(&token, &jwks(vec![key.public_jwk(Some("k"))])).expect("same issuer");
    }

    #[test]
    fn wrong_audience_is_rejected() {
        let key = TestKey::es256();
        let mut c = claims();
        c["aud"] = json!(["someone-else"]);
        let token = key.sign(Some("k"), &c);
        assert!(verify_with_jwks(&token, &jwks(vec![key.public_jwk(Some("k"))])).is_err());
    }

    #[test]
    fn audience_array_containing_client_is_accepted() {
        let key = TestKey::es256();
        let mut c = claims();
        c["aud"] = json!(["other", CLIENT_ID]);
        let token = key.sign(Some("k"), &c);
        verify_with_jwks(&token, &jwks(vec![key.public_jwk(Some("k"))])).expect("aud array");
    }

    #[test]
    fn wrong_or_missing_nonce_is_rejected() {
        let key = TestKey::es256();
        let set = jwks(vec![key.public_jwk(Some("k"))]);
        let mut wrong = claims();
        wrong["nonce"] = json!("attacker-nonce");
        assert!(verify_with_jwks(&key.sign(Some("k"), &wrong), &set).is_err());
        let mut missing = claims();
        missing.as_object_mut().unwrap().remove("nonce");
        assert!(verify_with_jwks(&key.sign(Some("k"), &missing), &set).is_err());
    }

    #[test]
    fn alg_must_be_advertised_when_provider_lists_algs() {
        let advertised = vec!["RS256".to_string()];
        assert!(ensure_alg_advertised(Algorithm::RS256, &advertised).is_ok());
        assert!(ensure_alg_advertised(Algorithm::HS256, &advertised).is_err());
        assert!(ensure_alg_advertised(Algorithm::HS256, &[]).is_ok());
    }
}

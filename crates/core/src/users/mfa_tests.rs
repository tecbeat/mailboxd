//
// Copyright (c) 2026 tecbeat
//
// This file is part of mailboxd, a fork of the Bichon email archiving
// project. Modifications by tecbeat, 2026.
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

use super::*;
use crate::database::{find_impl, upsert_impl};
use crate::store::tantivy::envelope::test_support::{init_globals, unique_id};
use crate::users::{LoginResult, DEFAULT_ADMIN_ROLE_ID};

/// RFC 6238 Appendix B: the ASCII secret "12345678901234567890".
const RFC_SECRET: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

fn rfc_totp() -> Totp {
    build_totp(RFC_SECRET, "rfc@example.com").unwrap()
}

// ── Codes ────────────────────────────────────────────────────────────────

#[test]
fn generates_the_rfc_6238_sha1_test_vectors() {
    // The RFC lists 8 digits; a 6-digit code is the last six of them.
    for (time, code) in [
        (59, "287082"),
        (1_111_111_109, "081804"),
        (1_111_111_111, "050471"),
        (1_234_567_890, "005924"),
        (2_000_000_000, "279037"),
        (20_000_000_000, "353130"),
    ] {
        assert_eq!(rfc_totp().generate(time).to_string(), code, "time {time}");
        assert_eq!(
            accepted_step(&rfc_totp(), code, time, None),
            Some(time / 30),
            "time {time}"
        );
    }
}

#[test]
fn accepts_one_step_of_clock_drift_either_way_and_no_more() {
    let totp = rfc_totp();
    let now = 1_111_111_111;
    let step = now / 30;
    let code_at = |offset: i64| totp.generate((now as i64 + offset * 30) as u64).to_string();

    assert_eq!(
        accepted_step(&totp, &code_at(-1), now, None),
        Some(step - 1)
    );
    assert_eq!(accepted_step(&totp, &code_at(1), now, None), Some(step + 1));
    assert_eq!(accepted_step(&totp, &code_at(-2), now, None), None);
    assert_eq!(accepted_step(&totp, &code_at(2), now, None), None);
}

#[test]
fn accepts_each_time_step_only_once() {
    let totp = rfc_totp();
    let now = 1_111_111_111;
    let step = now / 30;
    let code = totp.generate(now).to_string();

    assert_eq!(accepted_step(&totp, &code, now, Some(step - 1)), Some(step));
    assert_eq!(accepted_step(&totp, &code, now, Some(step)), None);
    assert_eq!(accepted_step(&totp, &code, now, Some(step + 1)), None);
}

#[test]
fn accepts_only_complete_six_digit_codes() {
    let totp = rfc_totp();
    let now = 1_111_111_111;
    let code = totp.generate(now).to_string();
    // Upstream compared only as many digits as were sent.
    for partial in [&code[..1], &code[..5], &code[1..], ""] {
        assert_eq!(
            accepted_step(&totp, partial, now, None),
            None,
            "{partial:?}"
        );
    }
    assert_eq!(
        accepted_step(&totp, &format!("+{}", &code[1..]), now, None),
        None
    );
    assert_eq!(accepted_step(&totp, &format!("{code}0"), now, None), None);
}

#[test]
fn classifies_submitted_codes() {
    assert_eq!(classify_code("287082"), CodeInput::Totp("287082".into()));
    assert_eq!(classify_code(" 287 082 "), CodeInput::Totp("287082".into()));
    assert_eq!(
        classify_code("k7qm-3xpa"),
        CodeInput::Recovery("K7QM3XPA".into())
    );
    assert_eq!(
        classify_code("K7QM 3XPA"),
        CodeInput::Recovery("K7QM3XPA".into())
    );
    assert_eq!(
        classify_code("23456789"),
        CodeInput::Recovery("23456789".into())
    );
    for invalid in [
        "",
        "12345",
        "1234567",
        "28708a",
        "K7QM-3XP",
        "K7QM-3XPA1",
        "K7QM-3XP0",
    ] {
        assert_eq!(classify_code(invalid), CodeInput::Invalid, "{invalid:?}");
    }
}

#[test]
fn recovery_codes_are_ten_distinct_unambiguous_codes() {
    let codes = generate_recovery_codes().unwrap();
    assert_eq!(codes.len(), RECOVERY_CODE_COUNT);
    for code in &codes {
        let (left, right) = code.split_once('-').expect("XXXX-XXXX");
        assert_eq!((left.len(), right.len()), (4, 4), "{code}");
        assert!(
            format!("{left}{right}")
                .bytes()
                .all(|b| RECOVERY_ALPHABET.contains(&b)),
            "{code}"
        );
        assert_eq!(
            classify_code(code),
            CodeInput::Recovery(format!("{left}{right}"))
        );
    }
    let distinct: std::collections::HashSet<_> = codes.iter().collect();
    assert_eq!(distinct.len(), codes.len());
    assert_ne!(codes, generate_recovery_codes().unwrap());
}

#[test]
fn the_recovery_alphabet_has_no_lookalikes_or_duplicates() {
    for lookalike in b"0O1I" {
        assert!(!RECOVERY_ALPHABET.contains(lookalike));
    }
    let distinct: std::collections::HashSet<_> = RECOVERY_ALPHABET.iter().collect();
    assert_eq!(distinct.len(), RECOVERY_ALPHABET.len());
}

#[test]
fn hashed_recovery_codes_match_only_their_code() {
    let stored = hash_login_password("K7QM3XPA").unwrap();
    assert!(recovery_code_matches(&stored, "K7QM3XPA"));
    assert!(!recovery_code_matches(&stored, "K7QM3XPB"));
    assert!(!recovery_code_matches("not-a-hash", "K7QM3XPA"));
}

#[test]
fn random_bytes_differ_between_calls() {
    assert_ne!(
        random_bytes::<SECRET_BYTES>().unwrap(),
        random_bytes::<SECRET_BYTES>().unwrap()
    );
}

#[test]
fn labels_the_account_by_email_then_username_without_colons() {
    let user = UserModel {
        username: "ann".into(),
        email: "ann@example.com".into(),
        ..Default::default()
    };
    assert_eq!(account_label(&user), "ann@example.com");
    let no_email = UserModel {
        username: "ann:admin".into(),
        ..Default::default()
    };
    assert_eq!(account_label(&no_email), "ann_admin");
}

#[test]
fn the_otpauth_uri_names_issuer_and_account() {
    let uri = build_totp(RFC_SECRET, "ann@example.com")
        .unwrap()
        .to_url()
        .unwrap();
    assert!(
        uri.starts_with("otpauth://totp/mailboxd:ann%40example.com?"),
        "{uri}"
    );
    assert!(uri.contains(&format!("secret={RFC_SECRET}")), "{uri}");
    assert!(uri.contains("issuer=mailboxd"), "{uri}");
}

#[test]
fn rejects_an_invalid_stored_secret() {
    let err = build_totp("not base32!", "ann").unwrap_err();
    assert_eq!(err.code(), ErrorCode::InternalError);
    // Too short for RFC 4226 (fewer than 128 bits).
    let err = build_totp("GEZDGNBV", "ann").unwrap_err();
    assert_eq!(err.code(), ErrorCode::InternalError);
}

#[test]
fn names_the_second_factors() {
    assert_eq!(SecondFactor::Totp.as_str(), "totp");
    assert_eq!(SecondFactor::RecoveryCode.as_str(), "recovery_code");
}

#[test]
fn records_without_two_factor_state_load_with_it_off() {
    let user = UserModel {
        id: 7,
        username: "old".into(),
        ..Default::default()
    };
    let mut json = serde_json::to_value(&user).unwrap();
    json.as_object_mut().unwrap().remove("mfa");
    let loaded: UserModel = serde_json::from_value(json).unwrap();
    assert_eq!(loaded.mfa, MfaState::default());

    let partial: MfaState = serde_json::from_str(r#"{"totp_enabled":true}"#).unwrap();
    assert!(partial.totp_enabled);
    assert_eq!(partial.totp_secret, None);
    assert!(partial.recovery_codes.is_empty());
    assert_eq!(partial.last_step, None);
}

#[test]
fn login_results_mention_two_factor_only_when_pending() {
    let plain = serde_json::to_value(LoginResult {
        success: true,
        access_token: Some("t".into()),
        ..Default::default()
    })
    .unwrap();
    assert!(plain.get("mfa_required").is_none(), "{plain}");
    assert!(plain.get("mfa_challenge").is_none(), "{plain}");
    assert!(plain.get("error_code").is_none(), "{plain}");

    let pending = serde_json::to_value(LoginResult {
        success: true,
        mfa_required: true,
        mfa_challenge: Some("c".into()),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(pending["mfa_required"], true);
    assert_eq!(pending["mfa_challenge"], "c");

    let parsed: LoginResult = serde_json::from_str(r#"{"success":false}"#).unwrap();
    assert!(!parsed.mfa_required);
    assert_eq!(parsed.mfa_challenge, None);
}

#[test]
fn the_user_view_shows_whether_two_factor_is_on_and_nothing_else() {
    let user = UserModel {
        mfa: MfaState {
            totp_secret: Some("encrypted-secret".into()),
            totp_enabled: true,
            recovery_codes: vec!["recovery-hash".into()],
            last_step: Some(1),
        },
        ..Default::default()
    };
    let view = user.to_view(&Default::default());
    assert!(view.totp_enabled);
    let json = serde_json::to_string(&view).unwrap();
    assert!(!json.contains("encrypted-secret"), "{json}");
    assert!(!json.contains("recovery-hash"), "{json}");
}

// ── Users in the database ─────────────────────────────────────────────────

/// A stored local user; unique per run because the database persists.
fn stored_user(password: Option<&str>, sso_provider: Option<&str>) -> UserModel {
    init_globals();
    let id = unique_id();
    let user = UserModel {
        id,
        username: format!("mfa-{id}"),
        email: format!("mfa-{id}@example.com"),
        password: password.map(|p| hash_login_password(p).unwrap()),
        global_roles: vec![DEFAULT_ADMIN_ROLE_ID],
        sso_provider: sso_provider.map(Into::into),
        sso_id: sso_provider.map(|_| format!("subject-{id}")),
        theme: Some("dark".into()),
        ..Default::default()
    };
    upsert_impl(DB_MANAGER.db(), user.clone()).unwrap();
    user
}

fn reload(user_id: u64) -> UserModel {
    UserModel::find(user_id).unwrap().expect("user exists")
}

fn current_code(secret: &str) -> String {
    build_totp(secret, "test")
        .unwrap()
        .generate(now_secs())
        .to_string()
}

/// The code for the next time step: still inside the accepted drift, but
/// not yet used.
fn next_code(secret: &str) -> String {
    build_totp(secret, "test")
        .unwrap()
        .generate(now_secs() + TOTP_STEP_SECS)
        .to_string()
}

/// A well-formed code that no step inside the accepted drift produces.
fn wrong_code(secret: &str) -> String {
    let totp = build_totp(secret, "test").unwrap();
    let now = now_secs();
    let valid: Vec<String> = (0..=2 * TOTP_STEP_SECS)
        .step_by(TOTP_STEP_SECS as usize)
        .map(|offset| totp.generate(now + offset - TOTP_STEP_SECS).to_string())
        .collect();
    (0..1_000_000)
        .map(|n| format!("{n:06}"))
        .find(|code| !valid.contains(code))
        .unwrap()
}

/// A user with 2FA on: (user, base32 secret, recovery codes).
fn enrolled_user() -> (UserModel, String, Vec<String>) {
    let user = stored_user(Some("pw-s3cret"), None);
    let enrolment = UserModel::start_totp_enrolment(user.id).unwrap();
    let codes = UserModel::confirm_totp_enrolment(user.id, &current_code(&enrolment.secret))
        .unwrap()
        .expect("current code accepted");
    (reload(user.id), enrolment.secret, codes)
}

#[test]
fn enrolment_turns_two_factor_on_after_a_valid_code() {
    let user = stored_user(None, None);
    assert!(user.can_enrol_mfa());

    let enrolment = UserModel::start_totp_enrolment(user.id).unwrap();
    let pending = reload(user.id);
    assert!(!pending.mfa.totp_enabled, "on only after confirming");
    let stored_secret = pending.mfa.totp_secret.expect("secret stored");
    assert_ne!(stored_secret, enrolment.secret, "stored encrypted");
    assert_eq!(decrypt!(&stored_secret).unwrap(), enrolment.secret);
    assert_eq!(
        Secret::try_from_base32(&enrolment.secret)
            .unwrap()
            .as_bytes()
            .len(),
        SECRET_BYTES
    );
    assert!(
        enrolment.otpauth_uri.starts_with(&format!(
            "otpauth://totp/mailboxd:mfa-{}%40example.com?",
            user.id
        )),
        "{}",
        enrolment.otpauth_uri
    );
    assert!(enrolment
        .otpauth_uri
        .contains(&format!("secret={}", enrolment.secret)));

    // A wrong code changes nothing.
    let wrong = wrong_code(&enrolment.secret);
    assert_eq!(
        UserModel::confirm_totp_enrolment(user.id, &wrong).unwrap(),
        None
    );
    assert_eq!(
        UserModel::confirm_totp_enrolment(user.id, "nonsense").unwrap(),
        None
    );
    assert!(!reload(user.id).mfa.totp_enabled);

    let code = current_code(&enrolment.secret);
    let recovery = UserModel::confirm_totp_enrolment(user.id, &code)
        .unwrap()
        .expect("current code accepted");
    assert_eq!(recovery.len(), RECOVERY_CODE_COUNT);

    let enabled = reload(user.id).mfa;
    assert!(enabled.totp_enabled);
    assert_eq!(enabled.totp_secret.as_deref(), Some(stored_secret.as_str()));
    assert_eq!(enabled.recovery_codes.len(), RECOVERY_CODE_COUNT);
    for (hash, code) in enabled.recovery_codes.iter().zip(&recovery) {
        assert!(hash.starts_with("$argon2id$"));
        assert!(recovery_code_matches(hash, &normalize_recovery_code(code)));
    }
    assert!(enabled.last_step.is_some());

    // The confirming code cannot log in afterwards.
    assert_eq!(
        UserModel::verify_second_factor(user.id, &code).unwrap(),
        None
    );
}

#[test]
fn restarting_an_unconfirmed_enrolment_replaces_the_secret() {
    let user = stored_user(None, None);
    let first = UserModel::start_totp_enrolment(user.id).unwrap();
    let second = UserModel::start_totp_enrolment(user.id).unwrap();
    assert_ne!(first.secret, second.secret);
    let stored = reload(user.id).mfa.totp_secret.unwrap();
    assert_eq!(decrypt!(&stored).unwrap(), second.secret);
}

#[test]
fn confirming_without_a_started_enrolment_is_refused() {
    let user = stored_user(None, None);
    let err = UserModel::confirm_totp_enrolment(user.id, "123456").unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidParameter);
}

#[test]
fn enrolling_again_while_on_is_refused_and_keeps_two_factor() {
    let (user, secret, _) = enrolled_user();
    let before = reload(user.id).mfa;

    let err = UserModel::start_totp_enrolment(user.id).unwrap_err();
    assert_eq!(err.code(), ErrorCode::AlreadyExists);
    let err = UserModel::confirm_totp_enrolment(user.id, &current_code(&secret)).unwrap_err();
    assert_eq!(err.code(), ErrorCode::AlreadyExists);

    assert_eq!(reload(user.id).mfa, before);
}

#[test]
fn sso_users_cannot_enrol() {
    let user = stored_user(None, Some("oidc"));
    assert!(!user.can_enrol_mfa());
    let err = UserModel::start_totp_enrolment(user.id).unwrap_err();
    assert_eq!(err.code(), ErrorCode::Forbidden);
    let err = UserModel::confirm_totp_enrolment(user.id, "123456").unwrap_err();
    assert_eq!(err.code(), ErrorCode::Forbidden);
    assert_eq!(reload(user.id).mfa, MfaState::default());
}

#[test]
fn unknown_users_are_reported_as_not_found() {
    init_globals();
    let missing = unique_id();
    for err in [
        UserModel::start_totp_enrolment(missing).unwrap_err(),
        UserModel::confirm_totp_enrolment(missing, "123456").unwrap_err(),
        UserModel::verify_second_factor(missing, "123456").unwrap_err(),
        UserModel::disable_totp(missing, "123456").unwrap_err(),
        UserModel::reset_totp(missing).unwrap_err(),
    ] {
        assert_eq!(err.code(), ErrorCode::ResourceNotFound);
    }
    assert!(
        UserModel::find(missing).unwrap().is_none(),
        "nothing created"
    );
}

#[test]
fn a_totp_code_logs_in_once() {
    let (user, secret, _) = enrolled_user();
    let code = next_code(&secret);
    assert_eq!(
        UserModel::verify_second_factor(user.id, &code).unwrap(),
        Some(SecondFactor::Totp)
    );
    assert_eq!(
        UserModel::verify_second_factor(user.id, &code).unwrap(),
        None
    );
}

#[test]
fn a_recovery_code_logs_in_once_in_any_spelling() {
    let (user, _, codes) = enrolled_user();
    let spelled = codes[3].to_lowercase().replace('-', " ");
    assert_eq!(
        UserModel::verify_second_factor(user.id, &spelled).unwrap(),
        Some(SecondFactor::RecoveryCode)
    );
    assert_eq!(
        UserModel::verify_second_factor(user.id, &codes[3]).unwrap(),
        None
    );

    let left = reload(user.id).mfa.recovery_codes;
    assert_eq!(left.len(), RECOVERY_CODE_COUNT - 1);
    // The other codes still work.
    assert_eq!(
        UserModel::verify_second_factor(user.id, &codes[0]).unwrap(),
        Some(SecondFactor::RecoveryCode)
    );
}

#[test]
fn wrong_or_malformed_codes_do_not_log_in() {
    let (user, secret, _) = enrolled_user();
    for code in [wrong_code(&secret).as_str(), "ZZZZ-ZZZZ", "x", ""] {
        assert_eq!(
            UserModel::verify_second_factor(user.id, code).unwrap(),
            None,
            "{code:?}"
        );
    }
    assert_eq!(
        reload(user.id).mfa.recovery_codes.len(),
        RECOVERY_CODE_COUNT
    );
}

#[test]
fn no_code_logs_in_while_two_factor_is_off() {
    let user = stored_user(None, None);
    let enrolment = UserModel::start_totp_enrolment(user.id).unwrap();
    assert_eq!(
        UserModel::verify_second_factor(user.id, &current_code(&enrolment.secret)).unwrap(),
        None,
        "an unconfirmed enrolment is not a second factor"
    );
}

#[test]
fn an_enabled_state_without_a_secret_accepts_no_totp_code() {
    let user = stored_user(None, None);
    let mut broken = reload(user.id);
    broken.mfa.totp_enabled = true;
    upsert_impl(DB_MANAGER.db(), broken).unwrap();
    assert_eq!(
        UserModel::verify_second_factor(user.id, "123456").unwrap(),
        None
    );
}

#[test]
fn an_undecryptable_secret_is_an_error() {
    let user = stored_user(None, None);
    let mut broken = reload(user.id);
    broken.mfa.totp_enabled = true;
    broken.mfa.totp_secret = Some("not-encrypted".into());
    upsert_impl(DB_MANAGER.db(), broken).unwrap();
    assert!(UserModel::verify_second_factor(user.id, "123456").is_err());
}

#[test]
fn disabling_needs_a_valid_code() {
    let (user, _, codes) = enrolled_user();
    assert!(!UserModel::disable_totp(user.id, "ZZZZ-ZZZZ").unwrap());
    assert!(reload(user.id).mfa.totp_enabled);

    assert!(UserModel::disable_totp(user.id, &codes[1]).unwrap());
    assert_eq!(reload(user.id).mfa, MfaState::default());

    let err = UserModel::disable_totp(user.id, &codes[2]).unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidParameter);
}

#[test]
fn a_reset_removes_all_two_factor_state() {
    let (user, secret, _) = enrolled_user();
    UserModel::reset_totp(user.id).unwrap();
    let reset = reload(user.id);
    assert_eq!(reset.mfa, MfaState::default());
    assert_eq!(
        UserModel::verify_second_factor(user.id, &next_code(&secret)).unwrap(),
        None
    );
    // The user can enrol again.
    UserModel::start_totp_enrolment(user.id).unwrap();
}

// ── Login challenges ───────────────────────────────────────────────────────

fn challenge(token: &str) -> Option<MfaChallenge> {
    find_impl::<MfaChallenge>(DB_MANAGER.db(), token).unwrap()
}

#[test]
fn a_password_login_with_two_factor_returns_a_challenge_instead_of_a_token() {
    let (user, _, _) = enrolled_user();
    let result = UserModel::authenticate_user(user.username.clone(), "pw-s3cret".into()).unwrap();
    assert!(result.success);
    assert!(result.mfa_required);
    assert_eq!(result.access_token, None);
    assert_eq!(result.theme, None, "preferences come with the token");
    let token = result.mfa_challenge.expect("challenge issued");
    let stored = challenge(&token).expect("challenge stored");
    assert_eq!(stored.user_id, user.id);
    assert_eq!(stored.attempts, 0);
    let ttl = stored.expire_at - utc_now!();
    assert!(
        ttl > MFA_CHALLENGE_TTL_MS - 60_000 && ttl <= MFA_CHALLENGE_TTL_MS,
        "{ttl}"
    );

    // A wrong password gets no challenge.
    let wrong = UserModel::authenticate_user(user.username, "wrong".into()).unwrap();
    assert!(!wrong.success);
    assert!(!wrong.mfa_required);
    assert_eq!(wrong.mfa_challenge, None);
}

#[test]
fn a_password_login_without_two_factor_returns_the_token() {
    let user = stored_user(Some("plain-s3cret"), None);
    let result = UserModel::authenticate_user(user.username, "plain-s3cret".into()).unwrap();
    assert!(result.success);
    assert!(!result.mfa_required);
    assert!(result.access_token.is_some());
    assert_eq!(result.theme.as_deref(), Some("dark"));
}

#[test]
fn the_right_code_completes_the_login_once() {
    let (user, secret, _) = enrolled_user();
    let token = MfaChallenge::issue(user.id).unwrap();

    let MfaVerification::Success {
        user: logged_in,
        factor,
        access_token,
    } = MfaChallenge::verify(&token, &next_code(&secret)).unwrap()
    else {
        panic!("expected success");
    };
    assert_eq!(logged_in.id, user.id);
    assert_eq!(logged_in.theme.as_deref(), Some("dark"));
    assert_eq!(factor, SecondFactor::Totp);
    let resolved = AccessTokenModel::resolve_user_from_token(&access_token).unwrap();
    assert_eq!(resolved.id, user.id);

    assert_eq!(challenge(&token), None, "challenge consumed");
    assert_eq!(
        MfaChallenge::verify(&token, &next_code(&secret)).unwrap(),
        MfaVerification::InvalidChallenge
    );
}

#[test]
fn a_recovery_code_completes_the_login() {
    let (user, _, codes) = enrolled_user();
    let token = MfaChallenge::issue(user.id).unwrap();
    let outcome = MfaChallenge::verify(&token, &codes[5]).unwrap();
    assert!(matches!(
        outcome,
        MfaVerification::Success {
            factor: SecondFactor::RecoveryCode,
            ..
        }
    ));
}

#[test]
fn a_challenge_allows_five_wrong_codes() {
    let (user, secret, _) = enrolled_user();
    let token = MfaChallenge::issue(user.id).unwrap();

    let attempts = [
        ("x", None),
        ("ZZZZ-ZZZZ", Some(SecondFactor::RecoveryCode)),
        ("", None),
        ("abc", None),
        ("-", None),
    ];
    for (n, (code, attempted)) in attempts.into_iter().enumerate() {
        let left = MFA_MAX_ATTEMPTS - 1 - n as u32;
        assert_eq!(
            MfaChallenge::verify(&token, code).unwrap(),
            MfaVerification::WrongCode {
                user: Box::new(reload(user.id)),
                attempted,
                attempts_left: left,
            },
            "attempt {n}"
        );
        assert_eq!(challenge(&token).is_some(), left > 0, "attempt {n}");
    }

    // Even the right code is too late now.
    assert_eq!(
        MfaChallenge::verify(&token, &next_code(&secret)).unwrap(),
        MfaVerification::InvalidChallenge
    );
}

#[test]
fn a_wrong_totp_code_is_reported_as_a_totp_attempt() {
    let (user, secret, _) = enrolled_user();
    let token = MfaChallenge::issue(user.id).unwrap();
    let outcome = MfaChallenge::verify(&token, &wrong_code(&secret)).unwrap();
    assert!(
        matches!(
            outcome,
            MfaVerification::WrongCode {
                attempted: Some(SecondFactor::Totp),
                attempts_left: 4,
                ..
            }
        ),
        "{outcome:?}"
    );
}

#[test]
fn a_challenge_attempt_is_counted_before_the_code_is_checked() {
    let (user, _, _) = enrolled_user();
    let token = MfaChallenge::issue(user.id).unwrap();
    let mut used_up = challenge(&token).unwrap();
    used_up.attempts = MFA_MAX_ATTEMPTS;
    upsert_impl(DB_MANAGER.db(), used_up).unwrap();
    assert_eq!(
        MfaChallenge::verify(&token, "x").unwrap(),
        MfaVerification::InvalidChallenge
    );
    assert_eq!(challenge(&token), None);
}

#[test]
fn expired_unknown_and_orphaned_challenges_are_invalid() {
    let (user, secret, _) = enrolled_user();

    let token = MfaChallenge::issue(user.id).unwrap();
    let mut expired = challenge(&token).unwrap();
    expired.expire_at = utc_now!() - 1;
    upsert_impl(DB_MANAGER.db(), expired).unwrap();
    assert_eq!(
        MfaChallenge::verify(&token, &next_code(&secret)).unwrap(),
        MfaVerification::InvalidChallenge
    );
    assert_eq!(challenge(&token), None, "expired challenge removed");

    assert_eq!(
        MfaChallenge::verify("no-such-challenge", "123456").unwrap(),
        MfaVerification::InvalidChallenge
    );

    let orphan = MfaChallenge::issue(unique_id()).unwrap();
    assert_eq!(
        MfaChallenge::verify(&orphan, "123456").unwrap(),
        MfaVerification::InvalidChallenge
    );
    assert_eq!(challenge(&orphan), None);

    let without_mfa = stored_user(None, None);
    let token = MfaChallenge::issue(without_mfa.id).unwrap();
    assert_eq!(
        MfaChallenge::verify(&token, "123456").unwrap(),
        MfaVerification::InvalidChallenge
    );
    assert_eq!(challenge(&token), None);
}

#[test]
fn issuing_a_challenge_removes_expired_ones() {
    init_globals();
    let stale = MfaChallenge {
        token: format!("stale-{}", unique_id()),
        user_id: unique_id(),
        expire_at: utc_now!() - 1,
        attempts: 0,
    };
    upsert_impl(DB_MANAGER.db(), stale.clone()).unwrap();
    let fresh = MfaChallenge::issue(unique_id()).unwrap();
    assert_eq!(challenge(&stale.token), None);
    assert!(challenge(&fresh).is_some());
}

#[test]
fn a_verification_error_surfaces_as_an_error() {
    let user = stored_user(None, None);
    let mut broken = reload(user.id);
    broken.mfa.totp_enabled = true;
    broken.mfa.totp_secret = Some("not-encrypted".into());
    upsert_impl(DB_MANAGER.db(), broken).unwrap();
    let token = MfaChallenge::issue(user.id).unwrap();
    assert!(MfaChallenge::verify(&token, "123456").is_err());
}

// ── Guards against parallel changes ─────────────────────────────────────────

fn with_mfa(mfa: MfaState) -> UserModel {
    UserModel {
        id: 9,
        mfa,
        ..Default::default()
    }
}

#[test]
fn beginning_an_enrolment_replaces_any_unconfirmed_state() {
    let (stored, outcome) = begin_enrolment(
        with_mfa(MfaState {
            totp_secret: Some("old".into()),
            recovery_codes: vec!["stale".into()],
            last_step: Some(3),
            ..Default::default()
        }),
        "new".into(),
    );
    outcome.unwrap();
    assert_eq!(
        stored.mfa,
        MfaState {
            totp_secret: Some("new".into()),
            ..Default::default()
        }
    );
    assert!(stored.updated_at > 0);
}

#[test]
fn beginning_an_enrolment_keeps_two_factor_that_was_turned_on_meanwhile() {
    let enabled = MfaState {
        totp_secret: Some("active".into()),
        totp_enabled: true,
        ..Default::default()
    };
    let (stored, outcome) = begin_enrolment(with_mfa(enabled.clone()), "new".into());
    assert_eq!(outcome.unwrap_err().code(), ErrorCode::AlreadyExists);
    assert_eq!(stored.mfa, enabled);
}

#[test]
fn finishing_an_enrolment_turns_two_factor_on_for_the_checked_secret() {
    let (stored, outcome) = finish_enrolment(
        with_mfa(MfaState {
            totp_secret: Some("secret".into()),
            ..Default::default()
        }),
        "secret",
        vec!["hash".into()],
        42,
    );
    outcome.unwrap();
    assert_eq!(
        stored.mfa,
        MfaState {
            totp_secret: Some("secret".into()),
            totp_enabled: true,
            recovery_codes: vec!["hash".into()],
            last_step: Some(42),
        }
    );
}

#[test]
fn finishing_an_enrolment_refuses_a_parallel_change() {
    let restarted = MfaState {
        totp_secret: Some("restarted".into()),
        ..Default::default()
    };
    let (stored, outcome) = finish_enrolment(with_mfa(restarted.clone()), "checked", vec![], 42);
    assert_eq!(outcome.unwrap_err().code(), ErrorCode::InvalidParameter);
    assert_eq!(stored.mfa, restarted);

    let enabled = MfaState {
        totp_secret: Some("checked".into()),
        totp_enabled: true,
        last_step: Some(41),
        ..Default::default()
    };
    let (stored, outcome) = finish_enrolment(with_mfa(enabled.clone()), "checked", vec![], 42);
    assert_eq!(outcome.unwrap_err().code(), ErrorCode::AlreadyExists);
    assert_eq!(stored.mfa, enabled);
}

#[test]
fn an_update_of_a_missing_user_is_not_found_and_errors_pass_through() {
    assert_eq!(updated(1, Ok(Some(5))).unwrap(), 5);
    assert_eq!(
        updated::<()>(1, Ok(None)).unwrap_err().code(),
        ErrorCode::ResourceNotFound
    );
    let failure = raise_error!("disk full".into(), ErrorCode::InternalError);
    assert_eq!(
        updated::<()>(1, Err(failure)).unwrap_err().code(),
        ErrorCode::InternalError
    );
}

#[test]
fn an_account_name_with_a_colon_is_refused() {
    // Why account_label() replaces colons: otpauth uses ':' as separator.
    let err = build_totp(RFC_SECRET, "ann:admin").unwrap_err();
    assert_eq!(err.code(), ErrorCode::InternalError);
    let totp = build_totp(RFC_SECRET, "ann_admin").unwrap();
    assert!(otpauth_uri(&totp)
        .unwrap()
        .starts_with("otpauth://totp/mailboxd:ann_admin?"));
}

#[test]
fn a_user_without_a_password_gets_no_challenge() {
    let user = stored_user(None, None);
    let mut enabled = reload(user.id);
    enabled.mfa.totp_enabled = true;
    upsert_impl(DB_MANAGER.db(), enabled).unwrap();
    let result = UserModel::authenticate_user(user.username, "anything".into()).unwrap();
    assert!(!result.success);
    assert!(!result.mfa_required);
    assert_eq!(result.mfa_challenge, None);
}

#[test]
fn an_auto_provisioned_sso_user_starts_without_two_factor_and_cannot_enrol() {
    init_globals();
    let id = unique_id();
    let user = UserModel::find_or_provision_sso_user(
        "oidc",
        &format!("subject-{id}"),
        &format!("sso-{id}@example.com"),
        None,
        DEFAULT_ADMIN_ROLE_ID,
    )
    .unwrap();
    assert_eq!(user.mfa, MfaState::default());
    assert!(!user.can_enrol_mfa());
    assert_eq!(reload(user.id).mfa, MfaState::default());
}

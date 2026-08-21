use std::path::{Path, PathBuf};

use mailboxd_blob::{Codec, Config, Engine};
use mailboxd_core::{
    error::{code::ErrorCode, MailboxdResult},
    migrate::{read_storage_version, write_storage_version, CURRENT_STORAGE_VERSION},
    raise_error,
};
use console::style;
use dialoguer::{theme::ColorfulTheme, Input};
use fjall::{Config as FjallConfig, Database};
use indicatif::{ProgressBar, ProgressStyle};

fn hex_key_to_raw(hex_bytes: &[u8]) -> MailboxdResult<[u8; 32]> {
    let hex_str = std::str::from_utf8(hex_bytes).map_err(|e| {
        raise_error!(
            format!("invalid UTF-8 in fjall key: {e:#?}"),
            ErrorCode::InternalError
        )
    })?;
    let mut raw = [0u8; 32];
    hex::decode_to_slice(hex_str, &mut raw).map_err(|e| {
        raise_error!(
            format!("invalid hex in fjall key '{hex_str}': {e:#?}"),
            ErrorCode::InternalError
        )
    })?;
    Ok(raw)
}

fn migrate_keyspace(
    engine: &Engine,
    db: &Database,
    ks_name: &str,
    label: &str,
    batch_size: usize,
) -> MailboxdResult<u64> {
    let ks = db
        .keyspace(ks_name, || {
            panic!("{ks_name} keyspace not found in fjall database")
        })
        .map_err(|e| {
            raise_error!(
                format!("failed to open fjall keyspace '{ks_name}': {e:#?}"),
                ErrorCode::InternalError
            )
        })?;

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg} [{elapsed_precise}]").unwrap(),
    );
    pb.set_message(format!("Scanning {label} blobs..."));

    let mut count: u64 = 0;
    let mut batch: Vec<([u8; 32], Vec<u8>, Codec)> = Vec::with_capacity(batch_size);

    for item in ks.iter() {
        let (key_bytes, value) = item.into_inner().map_err(|e| {
            raise_error!(
                format!("fjall iter error in '{ks_name}': {e:#?}"),
                ErrorCode::InternalError
            )
        })?;

        if value.is_empty() {
            continue;
        }
        // MAX_VALUE_SIZE = 100 MB (mailboxd_blob::types)
        if value.len() > 100 * 1024 * 1024 {
            let raw_key = hex_key_to_raw(&key_bytes)?;
            eprintln!(
                "{}",
                console::style(format!(
                    "WARN: skipping oversized blob key={} ({} bytes)",
                    hex::encode(raw_key),
                    value.len()
                ))
                .yellow()
            );
            continue;
        }
        let raw_key = hex_key_to_raw(&key_bytes)?;
        batch.push((raw_key, value.to_vec(), Codec::Zstd));

        if batch.len() >= batch_size {
            engine
                .put_batch(&batch)
                .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;
            count += batch.len() as u64;
            pb.set_message(format!("{label}: {} blobs migrated...", count));
            batch.clear();
        }
    }

    if !batch.is_empty() {
        engine
            .put_batch(&batch)
            .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;
        count += batch.len() as u64;
    }

    pb.finish_with_message(format!("{label}: {} blobs migrated", count));
    Ok(count)
}

/// Number of blobs moved by a successful [`migrate_v1_to_v2`] run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MigrationOutcome {
    pub emails: u64,
    pub attachments: u64,
}

/// Return `true` if `path` is a directory that contains at least one entry.
fn dir_has_entries(path: &Path) -> MailboxdResult<bool> {
    if !path.is_dir() {
        return Ok(false);
    }
    let mut entries = std::fs::read_dir(path)
        .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;
    Ok(entries.next().is_some())
}

/// Non-interactive, idempotent migration of a v1.x fjall blob store to the v2.x
/// mailboxd-blob engine. This is the shared engine behind both the interactive
/// admin menu and the server's unattended startup migration.
///
/// * `root_dir` — `MAILBOXD_ROOT_DIR`, where the `STORAGE_VERSION` marker lives.
/// * `data_base` — directory that contains `mailboxd-storage` (equal to
///   `root_dir` unless `MAILBOXD_DATA_DIR` is set).
/// * `batch_size` — number of blobs written per `put_batch` call.
///
/// Returns `Ok(None)` when there is nothing to do and `Ok(Some(outcome))` after
/// a successful conversion. It is **non-destructive** and safe to run on every
/// start: the source fjall data is never modified or deleted, and it never
/// touches an existing `blobs/` directory (which may hold a live v2 store, e.g.
/// an adopted Bichon volume). The following states are treated as no-ops:
///
/// * `STORAGE_VERSION` already reports v2 or higher (migration completed, or a
///   fresh v2 install),
/// * a `blobs/` directory already exists (live v2 store, or an interrupted
///   earlier migration that an operator must resolve manually),
/// * `mailboxd-storage` is missing or empty (fresh install), or
/// * `mailboxd-storage` is not a fjall database (e.g. a legacy v0.3.7 layout,
///   which needs the interactive tool).
pub fn migrate_v1_to_v2(
    root_dir: &Path,
    data_base: &Path,
    batch_size: usize,
) -> MailboxdResult<Option<MigrationOutcome>> {
    // Already on v2 — migration done, or a fresh v2 install.
    if read_storage_version(root_dir).is_some_and(|v| v >= CURRENT_STORAGE_VERSION) {
        return Ok(None);
    }

    let fjall_path = data_base.join("mailboxd-storage");
    let blob_path = fjall_path.join("blobs");

    // A blobs/ directory already exists. Either a live v2 store whose version
    // marker was never written (freshly adopted Bichon volume) or an
    // interrupted earlier migration. Never delete data unattended — leave it
    // for the operator to resolve via the interactive tool.
    if blob_path.exists() {
        return Ok(None);
    }

    // No storage yet → fresh install; let the server initialise it.
    if !dir_has_entries(&fjall_path)? {
        return Ok(None);
    }

    // Open the legacy fjall database. A non-fjall layout (e.g. legacy v0.3.7
    // Tantivy storage) cannot be handled here.
    let db = match Database::open(FjallConfig::new(&fjall_path)) {
        Ok(db) => db,
        Err(_) => return Ok(None),
    };

    // Open the new mailboxd-blob engine at the exact path the runtime reads
    // from (`mailboxd-storage/blobs`).
    let mut config = Config::default();
    config.default_codec = Codec::Zstd;
    config.compress_threshold = 1024;
    config.flush_interval_secs = 0;
    config.gc_interval_secs = 0;
    let engine = Engine::open(&blob_path, config).map_err(|e| {
        raise_error!(
            format!("failed to open mailboxd-blob engine: {e:#?}"),
            ErrorCode::InternalError
        )
    })?;

    // Only migrate keyspaces that actually exist so installs without
    // attachments do not trip over a missing keyspace.
    let emails = if db.keyspace_exists("email") {
        migrate_keyspace(&engine, &db, "email", "Email", batch_size)?
    } else {
        0
    };
    let attachments = if db.keyspace_exists("attachments") {
        migrate_keyspace(&engine, &db, "attachments", "Attachment", batch_size)?
    } else {
        0
    };

    engine
        .flush()
        .map_err(|e| raise_error!(format!("blob flush failed: {e:#?}"), ErrorCode::InternalError))?;
    engine.shutdown().map_err(|e| {
        raise_error!(format!("blob shutdown failed: {e:#?}"), ErrorCode::InternalError)
    })?;

    write_storage_version(root_dir, CURRENT_STORAGE_VERSION).map_err(|e| {
        raise_error!(
            format!("failed to write STORAGE_VERSION: {e:#?}"),
            ErrorCode::InternalError
        )
    })?;

    Ok(Some(MigrationOutcome {
        emails,
        attachments,
    }))
}

pub fn handle_migrate_v1(theme: &ColorfulTheme) {
    println!(
        "\n{}",
        style("MIGRATION: Mailboxd v1.x Storage → v2.x (Fjall → mailboxd-blob)")
            .bold()
            .yellow()
    );
    println!(
        "{}\n",
        style(
            "This migrates blob storage from the fjall engine to mailboxd-blob.\n\
              Tantivy indexes and metadata (memdb) are NOT affected."
        )
        .dim()
    );

    let root_dir: String = Input::with_theme(theme)
        .with_prompt("Enter --mailboxd-root-dir (same value used by the old server)")
        .validate_with(|input: &String| -> Result<(), &str> {
            let path = PathBuf::from(input);
            if !path.is_absolute() {
                return Err("Path must be absolute.");
            }
            if !path.exists() {
                return Err("Directory does not exist.");
            }
            Ok(())
        })
        .interact_text()
        .unwrap();
    let root_dir = PathBuf::from(root_dir.trim());

    let data_base = {
        let input: String = Input::with_theme(theme)
            .with_prompt("Enter --mailboxd-data-dir (leave blank to use root directory)")
            .allow_empty(true)
            .interact_text()
            .unwrap();
        if input.trim().is_empty() {
            root_dir.clone()
        } else {
            let path = PathBuf::from(input.trim());
            if !path.exists() {
                eprintln!(
                    "{}",
                    style(format!("Data directory does not exist: {}", path.display())).red()
                );
                return;
            }
            path
        }
    };

    let fjall_path = data_base.join("mailboxd-storage");
    let blob_path = fjall_path.join("blobs");

    if !fjall_path.exists() {
        println!(
            "{}",
            style(format!(
                "Fjall database not found at '{}'. Is this really a v1.x install?",
                fjall_path.display()
            ))
            .red()
        );
        return;
    }

    if blob_path.exists() {
        println!(
            "{}",
            style(format!(
                "Target blob directory '{}' already exists.\n\
                 If you have already migrated, you can remove the old fjall files manually.\n\
                 Otherwise, delete this directory and re-run the migration.",
                blob_path.display()
            ))
            .yellow()
        );
        return;
    }

    let batch_size: usize = {
        let input: String = Input::with_theme(theme)
            .with_prompt(
                "Enter batch size (affects memory usage, higher = faster but uses more RAM)",
            )
            .default("1000".to_string())
            .validate_with(|s: &String| match s.trim().parse::<usize>() {
                Ok(n) if n > 0 => Ok(()),
                _ => Err("Please enter a valid positive number"),
            })
            .interact_text()
            .unwrap_or("1000".to_string());
        input.trim().parse::<usize>().unwrap_or(1000)
    };

    println!(
        "{} Using batch size: {}\n",
        style("✓").green(),
        style(batch_size).cyan().bold()
    );

    // Delegate the actual conversion to the shared, non-interactive engine so
    // the interactive tool and the server's unattended startup migration stay
    // in lockstep.
    println!("\n{}", style("Migrating fjall → mailboxd-blob...").dim());
    match migrate_v1_to_v2(&root_dir, &data_base, batch_size) {
        Ok(Some(outcome)) => {
            println!(
                "\n{}",
                style(format!(
                    "✅ Migration complete!\n\
                 \n\
                 📊 {} email blobs, {} attachment blobs migrated\n\
                 \n\
                 📖 **Next steps:**\n\
                 Refer to the official migration guide for:\n\
                 • How to verify the new storage\n\
                 • Cleanup commands for legacy files\n\
                 • Rollback instructions if needed\n\
                 \n\
                 🔗 {}\n\
                 \n\
                 ⚠️  **Important:** Old data is preserved until you manually remove it.\n\
                 Do not delete anything until you have verified the new server works correctly.",
                    outcome.emails,
                    outcome.attachments,
                    "https://git.teccave.de/tecbeat/mailboxd/wiki/Mailboxd-v2.x-Migration-Guide"
                ))
                .green()
                .bold()
            );
        }
        Ok(None) => {
            println!(
                "{}",
                style(
                    "Nothing to migrate: storage is already on v2, or a 'blobs' directory \
                     already exists. If an earlier migration was interrupted, remove the \
                     'blobs' directory and re-run."
                )
                .yellow()
            );
        }
        Err(e) => {
            println!("{}", style(format!("Migration failed: {e:#?}")).red());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mailboxd_core::utils::compute_content_hash;

    // ── hex_key_to_raw ────────────────────────────────────────────────

    #[test]
    fn hex_key_to_raw_valid() {
        // "hello" blake3 hex = 64 chars
        let hash_hex = compute_content_hash(b"hello");
        assert_eq!(hash_hex.len(), 64);

        let raw = hex_key_to_raw(hash_hex.as_bytes()).unwrap();
        // Decoding 64 hex chars → 32 bytes
        assert_eq!(raw.len(), 32);
        // Round-trip: raw → hex should match original
        assert_eq!(hex::encode(raw), hash_hex);
    }

    #[test]
    fn hex_key_to_raw_invalid_utf8() {
        // 0xFF is not valid UTF-8
        let invalid = vec![0xFFu8; 64];
        let err = hex_key_to_raw(&invalid).unwrap_err();
        assert!(err.to_string().contains("invalid UTF-8"));
    }

    #[test]
    fn hex_key_to_raw_invalid_hex() {
        // "zz" is valid UTF-8 but not valid hex
        let invalid = b"zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz";
        let err = hex_key_to_raw(invalid).unwrap_err();
        assert!(err.to_string().contains("invalid hex"));
    }

    #[test]
    fn hex_key_to_raw_wrong_length() {
        let short = b"abcd";
        let err = hex_key_to_raw(short).unwrap_err();
        assert!(err.to_string().contains("invalid hex"));
    }

    #[test]
    fn hex_key_to_raw_different_content() {
        let a = hex_key_to_raw(compute_content_hash(b"a").as_bytes()).unwrap();
        let b = hex_key_to_raw(compute_content_hash(b"b").as_bytes()).unwrap();
        assert_ne!(a, b);
    }

    // ── migrate_keyspace integration ──────────────────────────────────

    #[test]
    fn migrate_keyspace_end_to_end() {
        let tmp = tempfile::tempdir().unwrap();
        let fjall_path = tmp.path().join("fjall");
        let blob_path = tmp.path().join("blobs");

        use fjall::KeyspaceCreateOptions;

        // --- Setup: create a Fjall database with test blobs ---
        let fjall_db = Database::open(FjallConfig::new(&fjall_path)).unwrap();
        let ks = fjall_db
            .keyspace("test_ks", KeyspaceCreateOptions::default)
            .unwrap();

        // Insert test blobs using hex string keys (matching v1.x convention)
        let mut expected: Vec<(String, Vec<u8>)> = Vec::new();
        for i in 0..10 {
            let data = format!("blob data {}", i).into_bytes();
            let hash = compute_content_hash(&data);
            ks.insert(hash.as_bytes(), data.clone()).unwrap();
            expected.push((hash, data));
        }

        // --- Setup: create mailboxd-blob engine and run migration ---
        {
            let mut config = Config::default();
            config.flush_interval_secs = 0;
            config.gc_interval_secs = 0;
            let engine = Engine::open(&blob_path, config).unwrap();

            let count = migrate_keyspace(&engine, &fjall_db, "test_ks", "Test", 100).unwrap();
            assert_eq!(count, expected.len() as u64);

            engine.flush().unwrap();
            engine.shutdown().unwrap();
            // engine dropped here → LOCK released
        }

        // --- Verify: re-open engine and check all blobs ---
        let mut config = Config::default();
        config.flush_interval_secs = 0;
        config.gc_interval_secs = 0;
        let engine2 = Engine::open(&blob_path, config).unwrap();

        for (hex_hash, expected_data) in &expected {
            let mut raw_key = [0u8; 32];
            hex::decode_to_slice(hex_hash, &mut raw_key).unwrap();
            let got = engine2.get(&raw_key).unwrap();
            assert_eq!(
                got.as_deref(),
                Some(expected_data.as_slice()),
                "mismatch for key {}",
                hex_hash
            );
        }

        // Verify non-existent key returns None
        let fake_hash = compute_content_hash(b"nonexistent");
        let mut fake_key = [0u8; 32];
        hex::decode_to_slice(&fake_hash, &mut fake_key).unwrap();
        assert!(engine2.get(&fake_key).unwrap().is_none());

        engine2.shutdown().unwrap();
    }

    #[test]
    fn migrate_empty_keyspace() {
        let tmp = tempfile::tempdir().unwrap();
        let fjall_path = tmp.path().join("fjall");
        let blob_path = tmp.path().join("blobs");

        let fjall_db = Database::open(FjallConfig::new(&fjall_path)).unwrap();
        let _ks = fjall_db
            .keyspace("empty_ks", fjall::KeyspaceCreateOptions::default)
            .unwrap();

        let mut config = Config::default();
        config.flush_interval_secs = 0;
        config.gc_interval_secs = 0;
        let engine = Engine::open(&blob_path, config).unwrap();

        let count = migrate_keyspace(&engine, &fjall_db, "empty_ks", "Empty", 100).unwrap();
        assert_eq!(count, 0);

        engine.shutdown().unwrap();
    }

    #[test]
    fn hex_key_roundtrip_with_real_content_hash() {
        // Simulate the exact data flow from v1.x to v2.x
        let eml_data = b"From: sender@example.com\r\nSubject: Test\r\n\r\nHello world";
        let hash_hex = compute_content_hash(eml_data); // 64-char hex string

        // v1.x: key stored as hash_hex.as_bytes()
        let fjall_key = hash_hex.as_bytes().to_vec();
        assert_eq!(fjall_key.len(), 64);

        // Migration: hex decode → raw 32 bytes
        let raw_key = hex_key_to_raw(&fjall_key).unwrap();
        assert_eq!(raw_key.len(), 32);

        // v2.x: engine.put(raw_key, data)
        // Verify round-trip: raw_key → hex → compare
        let hex_roundtrip = hex::encode(raw_key);
        assert_eq!(hex_roundtrip, hash_hex);
    }

    // ── migrate_v1_to_v2: unattended startup migration ────────────────

    /// Build a realistic v1.x fjall volume at `<data_base>/mailboxd-storage`
    /// with the given number of email and attachment blobs, keyed the way the
    /// v1.x runtime keyed them (blake3 hex string). Returns the (hex_hash, data)
    /// pairs that must survive the migration.
    fn seed_v1_fjall_volume(
        data_base: &Path,
        emails: usize,
        attachments: usize,
    ) -> Vec<(String, Vec<u8>)> {
        use fjall::{KeyspaceCreateOptions, PersistMode};

        let storage = data_base.join("mailboxd-storage");
        let db = Database::open(FjallConfig::new(&storage)).unwrap();
        let mut expected = Vec::new();

        let email_ks = db.keyspace("email", KeyspaceCreateOptions::default).unwrap();
        for i in 0..emails {
            let data = format!("email body {i}").into_bytes();
            let hash = compute_content_hash(&data);
            email_ks.insert(hash.as_bytes(), data.clone()).unwrap();
            expected.push((hash, data));
        }

        if attachments > 0 {
            let att_ks = db
                .keyspace("attachments", KeyspaceCreateOptions::default)
                .unwrap();
            for i in 0..attachments {
                let data = format!("attachment payload {i}").into_bytes();
                let hash = compute_content_hash(&data);
                att_ks.insert(hash.as_bytes(), data.clone()).unwrap();
                expected.push((hash, data));
            }
        }

        db.persist(PersistMode::SyncAll).unwrap();
        // `db` is dropped here → the fjall lock is released so the migration
        // can open its own handle, exactly like the real cross-process flow.
        expected
    }

    fn assert_blobs_readable(storage: &Path, expected: &[(String, Vec<u8>)]) {
        let mut config = Config::default();
        config.flush_interval_secs = 0;
        config.gc_interval_secs = 0;
        let engine = Engine::open(&storage.join("blobs"), config).unwrap();
        for (hex_hash, data) in expected {
            let mut key = [0u8; 32];
            hex::decode_to_slice(hex_hash, &mut key).unwrap();
            assert_eq!(
                engine.get(&key).unwrap().as_deref(),
                Some(data.as_slice()),
                "blob {hex_hash} not readable after migration"
            );
        }
        engine.shutdown().unwrap();
    }

    #[test]
    fn migrate_v1_to_v2_converts_fjall_volume_and_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let storage = root.join("mailboxd-storage");

        let expected = seed_v1_fjall_volume(&root, 25, 5);

        // Pre-conditions of a broken existing v1.x instance.
        assert!(read_storage_version(&root).is_none());
        assert!(!storage.join("blobs").exists());

        // Unattended migration converts everything.
        let outcome = migrate_v1_to_v2(&root, &root, 8)
            .unwrap()
            .expect("v1.x volume should be migrated");
        assert_eq!(outcome.emails, 25);
        assert_eq!(outcome.attachments, 5);

        // The server's boot gate now passes.
        assert_eq!(read_storage_version(&root), Some(CURRENT_STORAGE_VERSION));

        // Every blob survives byte-for-byte, readable from the v2 engine.
        assert_blobs_readable(&storage, &expected);

        // Running again is a safe no-op (STORAGE_VERSION already v2).
        assert_eq!(migrate_v1_to_v2(&root, &root, 8).unwrap(), None);
        assert_blobs_readable(&storage, &expected);
    }

    #[test]
    fn migrate_v1_to_v2_handles_install_without_attachments() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();

        let expected = seed_v1_fjall_volume(&root, 3, 0);

        let outcome = migrate_v1_to_v2(&root, &root, 100)
            .unwrap()
            .expect("email-only volume should migrate");
        assert_eq!(outcome.emails, 3);
        assert_eq!(outcome.attachments, 0);
        assert_eq!(read_storage_version(&root), Some(CURRENT_STORAGE_VERSION));
        assert_blobs_readable(&root.join("mailboxd-storage"), &expected);
    }

    #[test]
    fn migrate_v1_to_v2_is_noop_on_fresh_install() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(
            migrate_v1_to_v2(tmp.path(), tmp.path(), 100).unwrap(),
            None
        );
    }

    #[test]
    fn migrate_v1_to_v2_is_noop_when_already_v2() {
        let tmp = tempfile::tempdir().unwrap();
        write_storage_version(tmp.path(), CURRENT_STORAGE_VERSION).unwrap();
        // A stray fjall directory must not tempt a re-migration.
        seed_v1_fjall_volume(tmp.path(), 2, 0);
        assert_eq!(
            migrate_v1_to_v2(tmp.path(), tmp.path(), 100).unwrap(),
            None
        );
    }

    #[test]
    fn migrate_v1_to_v2_never_touches_an_existing_v2_store() {
        // A live v2 store (e.g. a freshly adopted Bichon volume) whose
        // STORAGE_VERSION marker was never written must be left completely
        // untouched — this is the data-loss guard.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let blobs = root.join("mailboxd-storage").join("blobs");
        std::fs::create_dir_all(&blobs).unwrap();
        std::fs::write(blobs.join("0001.seg"), b"live v2 segment").unwrap();

        assert_eq!(migrate_v1_to_v2(&root, &root, 100).unwrap(), None);
        assert_eq!(
            std::fs::read(blobs.join("0001.seg")).unwrap(),
            b"live v2 segment"
        );
    }
}

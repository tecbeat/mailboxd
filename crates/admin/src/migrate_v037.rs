use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use mailboxd_core::{
    error::{code::ErrorCode, MailboxdResult},
    migrate::{
        is_tantivy_index_dir, read_storage_version, write_storage_version, CURRENT_STORAGE_VERSION,
    },
    raise_error,
};
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input};
use indicatif::{ProgressBar, ProgressStyle};
use tantivy::{
    collector::TopDocs,
    columnar::Column,
    query::TermQuery,
    schema::{IndexRecordOption, Value},
    DocAddress, Index, TantivyDocument, Term,
};

use crate::legacy::schema::SchemaTools;
use crate::migrate_store_v2::{NewDirs, NewIndexWriterV2};

pub struct LegacyDirs {
    pub envelope_dir: PathBuf,
    pub eml_dir: PathBuf,
}

impl LegacyDirs {
    pub fn new(index: PathBuf, data: PathBuf) -> Self {
        Self {
            envelope_dir: index,
            eml_dir: data,
        }
    }
}

pub fn is_legacy_data_layout_with_paths(
    envelope_dir: &PathBuf,
    eml_dir: &PathBuf,
) -> std::io::Result<bool> {
    let envelope_result = is_tantivy_index_dir(envelope_dir)?;
    let eml_result = is_tantivy_index_dir(eml_dir)?;
    Ok(envelope_result || eml_result)
}

/// Return the number of segments in the legacy EML Tantivy index.
pub fn count_eml_segments(legacy: &LegacyDirs) -> MailboxdResult<usize> {
    let eml_index = Index::open_in_dir(&legacy.eml_dir)
        .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;
    let reader = eml_index
        .reader()
        .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;
    let searcher = reader.searcher();
    Ok(searcher.segment_readers().len())
}

/// Resolved source and destination directories for a v0.3.7 → v2 migration.
///
/// The derivation matches both the interactive tool and
/// [`mailboxd_core::migrate::check_data_status`]: the legacy Tantivy indices
/// default to `root/envelope` and `root/eml`, and the new layout is written to
/// `<base>/mailboxd-indices` and `<base>/mailboxd-storage`, where `<base>` is
/// the corresponding `MAILBOXD_INDEX_DIR` / `MAILBOXD_DATA_DIR` if set, else the
/// root directory.
pub struct V037Paths {
    pub root: PathBuf,
    pub legacy_index: PathBuf,
    pub legacy_data: PathBuf,
    pub new_index: PathBuf,
    pub new_data: PathBuf,
}

impl V037Paths {
    pub fn derive(root: &Path, index_dir: Option<&Path>, data_dir: Option<&Path>) -> Self {
        let legacy_index = index_dir
            .map(Path::to_path_buf)
            .unwrap_or_else(|| root.join("envelope"));
        let legacy_data = data_dir
            .map(Path::to_path_buf)
            .unwrap_or_else(|| root.join("eml"));
        let new_index = index_dir
            .map(Path::to_path_buf)
            .unwrap_or_else(|| root.to_path_buf())
            .join("mailboxd-indices");
        let new_data = data_dir
            .map(Path::to_path_buf)
            .unwrap_or_else(|| root.to_path_buf())
            .join("mailboxd-storage");
        Self {
            root: root.to_path_buf(),
            legacy_index,
            legacy_data,
            new_index,
            new_data,
        }
    }
}

/// Result of a successful [`migrate_v037_to_v2`] run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct V037Outcome {
    pub migrated: usize,
    pub skipped: usize,
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

/// Non-interactive, guarded migration of a legacy v0.3.7 Tantivy layout to the
/// v2 mailboxd-blob layout. This is the shared engine behind the server's
/// unattended startup migration; the interactive [`handle_migration_v037`]
/// keeps its own prompts and per-segment progress reporting.
///
/// Returns `Ok(None)` when there is nothing to do (already on v2, or no legacy
/// v0.3.7 layout is present, so the caller can try another migration).
///
/// Unlike the fjall v1 migration this conversion is **not** re-runnable against
/// its own output — re-ingesting would duplicate Tantivy documents — so it only
/// runs on a clean legacy volume and writes `STORAGE_VERSION = 2` as its final,
/// committing step. If v2 target artifacts already exist without that marker
/// (an interrupted earlier run), it refuses with an error rather than risk a
/// doubled or partial migration; such a volume must be resolved with the
/// interactive tool.
pub fn migrate_v037_to_v2(
    paths: &V037Paths,
    batch_size: u32,
) -> MailboxdResult<Option<V037Outcome>> {
    // Already migrated, or a fresh v2 install.
    if read_storage_version(&paths.root).is_some_and(|v| v >= CURRENT_STORAGE_VERSION) {
        return Ok(None);
    }

    // Not a v0.3.7 layout — let the caller try other migrations.
    let is_legacy = is_legacy_data_layout_with_paths(&paths.legacy_index, &paths.legacy_data)
        .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;
    if !is_legacy {
        return Ok(None);
    }

    // A v2 target already exists without a completion marker: either an
    // interrupted earlier migration or an unexpected mixed layout. Re-running
    // the ingest would double the Tantivy documents, so refuse and defer to the
    // interactive tool rather than corrupt data unattended.
    let blob_dir = paths.new_data.join("blobs");
    if blob_dir.exists()
        || dir_has_entries(&paths.new_index)?
        || dir_has_entries(&paths.root.join("memdb"))?
    {
        return Err(raise_error!(
            format!(
                "v0.3.7 auto-migration aborted: v2 target artifacts already exist \
                 (blobs at '{}', new indices at '{}', or a memdb directory). This looks \
                 like an interrupted migration — resolve it with `mailboxd-admin` \
                 interactively.",
                blob_dir.display(),
                paths.new_index.display()
            ),
            ErrorCode::InternalError
        ));
    }

    // Step 1: metadata (meta.db + mailbox.db → memdb).
    crate::meta::migrate_metadata(&paths.root).map_err(|e| {
        raise_error!(
            format!("v0.3.7 metadata migration failed: {e}"),
            ErrorCode::InternalError
        )
    })?;

    // Step 2: email index + blob data, segment by segment. Reuses the exact
    // per-segment engine the interactive tool drives, with a callback that only
    // tallies the final per-segment counts.
    let legacy = LegacyDirs::new(paths.legacy_index.clone(), paths.legacy_data.clone());
    let total_segments = count_eml_segments(&legacy)?;

    let mut writer =
        NewIndexWriterV2::open(NewDirs::new(paths.new_index.clone(), paths.new_data.clone()))?;

    let mut migrated = 0usize;
    let mut skipped = 0usize;
    for seg_idx in 0..total_segments {
        let legacy = LegacyDirs::new(paths.legacy_index.clone(), paths.legacy_data.clone());
        do_migrate_segment_v2(batch_size, legacy, &mut writer, seg_idx, |msg| {
            if let Some(done) = msg.strip_prefix("DONE:") {
                let mut parts = done.split(':');
                migrated += parts
                    .next()
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(0);
                skipped += parts
                    .next()
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(0);
            }
        })?;
    }

    writer.finish_writers()?;
    writer.shutdown_engine()?;

    write_storage_version(&paths.root, CURRENT_STORAGE_VERSION).map_err(|e| {
        raise_error!(
            format!("failed to write STORAGE_VERSION: {e:#?}"),
            ErrorCode::InternalError
        )
    })?;

    Ok(Some(V037Outcome { migrated, skipped }))
}

pub fn handle_migration_v037(theme: &ColorfulTheme) {
    println!(
        "\n{}",
        style("MIGRATION: Mailboxd v0.3.7 Storage → v2.x (mailboxd-blob)")
            .bold()
            .yellow()
    );

    println!(
        "{}",
        style(
            "This tool migrates data from the legacy v0.3.7 Tantivy-based storage \
            architecture directly to the v2.x mailboxd-blob storage format."
        )
        .dim()
    );

    println!(
        "{}",
        style(
            "Legacy v0.3.7 architecture:\n\
            • envelope metadata stored in Tantivy\n\
            • message data stored in Tantivy\n\n\
                New v2.x architecture:\n\
            • mail indexes stored in Tantivy\n\
            • attachment indexes stored in Tantivy\n\
            • raw message data stored in mailboxd-blob engine\n\
            • attachment blobs stored in mailboxd-blob engine"
        )
        .dim()
    );

    println!(
        "\n{} {}",
        style("IMPORTANT:").yellow().bold(),
        style(
            "The paths below must exactly match what your old mailboxd server was configured with."
        )
        .yellow()
    );

    // --- mailboxd-root-dir ---
    let root_dir_str: String = Input::with_theme(theme)
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

    let root_path = PathBuf::from(&root_dir_str);

    // --- mailboxd-index-dir ---
    let default_index = root_path.join("envelope");
    let default_new_index = root_path.join("mailboxd-indices");
    let index_dir_str: String = Input::with_theme(theme)
        .with_prompt(format!(
            "Enter --mailboxd-index-dir (leave blank to use default: {})",
            style(default_index.display()).cyan()
        ))
        .allow_empty(true)
        .validate_with(|input: &String| -> Result<(), &str> {
            if input.is_empty() {
                return Ok(());
            }
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

    let index_path = if index_dir_str.is_empty() {
        default_index
    } else {
        PathBuf::from(&index_dir_str)
    };

    let new_index_path = if index_dir_str.is_empty() {
        default_new_index
    } else {
        PathBuf::from(&index_dir_str).join("mailboxd-indices")
    };

    // --- mailboxd-data-dir ---
    let default_data = root_path.join("eml");
    let default_new_data = root_path.join("mailboxd-storage");
    let data_dir_str: String = Input::with_theme(theme)
        .with_prompt(format!(
            "Enter --mailboxd-data-dir (leave blank to use default: {})",
            style(default_data.display()).cyan()
        ))
        .allow_empty(true)
        .validate_with(|input: &String| -> Result<(), &str> {
            if input.is_empty() {
                return Ok(());
            }
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

    let data_path = if data_dir_str.is_empty() {
        default_data
    } else {
        PathBuf::from(&data_dir_str)
    };

    let new_data_path = if data_dir_str.is_empty() {
        default_new_data
    } else {
        PathBuf::from(&data_dir_str).join("mailboxd-storage")
    };

    println!("\n{}", style("Paths to be migrated:").bold());
    println!("----------------------------------------");
    println!(
        "{:<20} : {}",
        "mailboxd-root-dir",
        style(root_path.display()).cyan()
    );
    println!(
        "{:<20} : {}",
        "mailboxd-index-dir",
        style(index_path.display()).cyan()
    );
    println!(
        "{:<20} : {}",
        "mailboxd-data-dir",
        style(data_path.display()).cyan()
    );
    println!("----------------------------------------");

    println!(
        "\n{} Checking legacy v0.3.7 storage layout...",
        style("⌛").yellow()
    );

    match is_legacy_data_layout_with_paths(&index_path, &data_path) {
        Ok(true) => {
            println!(
                "{} {}",
                style("✔").green(),
                style("Legacy v0.3.7 Tantivy-based storage detected. Migration to v2.x is required.")
                    .yellow()
            );
        }
        Ok(false) => {
            println!(
                "{} {}",
                style("✔").green(),
                style("No legacy v0.3.7 storage layout was detected at the specified paths.").green()
            );

            println!(
                "{}",
                style(
                    "The selected directories may already be using a newer storage architecture."
                )
                .dim()
            );

            return;
        }
        Err(e) => {
            eprintln!(
                "{} Failed to verify legacy storage layout: {:?}",
                style("ERROR:").red().bold(),
                e
            );

            std::process::exit(1);
        }
    }

    println!(
        "\n{} {}",
        style("⚠").yellow(),
        style(
            "This migration is non-destructive. Existing v0.x storage files will remain unchanged."
        )
        .yellow()
    );

    if !Confirm::with_theme(theme)
        .with_prompt("Ready to migrate?")
        .default(true)
        .interact()
        .unwrap()
    {
        println!("{}", style("Migration cancelled.").dim());
        return;
    }

    // Step 1: Migrate metadata (meta.db + mailbox.db → memdb)
    match crate::meta::migrate_metadata(&root_path) {
        Ok(()) => {}
        Err(e) => {
            eprintln!(
                "\n{} Metadata migration failed:\n{}",
                style("✘").red().bold(),
                style(e).red()
            );
            eprintln!(
                "{}",
                style("Aborting migration. No changes have been made to Tantivy data.").yellow()
            );
            return;
        }
    }

    println!(
        "\n{} {}",
        style("⌛").yellow(),
        style("Step 2: Migrating email index and blob data...").cyan()
    );

    println!(
        "\n{} {}",
        style("ℹ").blue(),
        style("Batch size controls memory usage during migration:").dim()
    );
    println!(
        "  {} 1000  — ~500MB RAM  (slower, low memory)",
        style("•").dim()
    );
    println!("  {} 3000  — ~1GB RAM    (recommended)", style("•").dim());
    println!(
        "  {} 5000  — ~2GB RAM    (faster, high memory)",
        style("•").dim()
    );
    println!(
        "  {} Note: actual memory usage depends on your average email size.",
        style("•").yellow()
    );
    println!(
        "  {}       If your mailbox contains many large attachments, use a smaller batch size.\n",
        style(" ").dim()
    );

    let batch_size: u32 = {
        let input: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter batch size (affects memory usage, see notes above)")
            .default("3000".to_string())
            .validate_with(|s: &String| match s.trim().parse::<usize>() {
                Ok(n) if n > 0 => Ok(()),
                _ => Err("Please enter a valid positive number"),
            })
            .interact_text()
            .unwrap_or("3000".to_string());
        input.trim().parse::<u32>().unwrap_or(3000)
    };

    println!(
        "{} Using batch size: {}\n",
        style("✓").green(),
        style(batch_size).cyan().bold()
    );

    let legacy = LegacyDirs::new(index_path.clone(), data_path.clone());
    let total_segments = match count_eml_segments(&legacy) {
        Ok(n) => n,
        Err(e) => {
            eprintln!(
                "\n{} Failed to count EML segments:\n{:?}",
                style("✘").red().bold(),
                e
            );
            return;
        }
    };

    if total_segments == 0 {
        println!(
            "{} {}",
            style("✔").green(),
            style("No EML segments found. Nothing to migrate.").bold()
        );
        return;
    }

    println!(
        "{} EML segments to migrate: {}",
        style("⌛").yellow(),
        style(total_segments).cyan()
    );

    let pb = ProgressBar::new(total_segments as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}",
            )
            .unwrap()
            .progress_chars("#>-"),
    );

    let mut writer = match NewIndexWriterV2::open(NewDirs::new(
        new_index_path.clone(),
        new_data_path.clone(),
    )) {
        Ok(w) => w,
        Err(e) => {
            pb.finish_with_message(format!("{}", style("Migration failed.").red()));
            eprintln!("\n{} {:?}", style("✘").red().bold(), e);
            return;
        }
    };

    let mut grand_total_migrated: usize = 0;
    let mut grand_total_skipped: usize = 0;

    for seg_idx in 0..total_segments {
        let seg_total: std::cell::Cell<usize> = std::cell::Cell::new(0);

        pb.set_message(format!("Segment {}/{}", seg_idx + 1, total_segments));
        let legacy = LegacyDirs::new(index_path.clone(), data_path.clone());
        match do_migrate_segment_v2(
            batch_size,
            legacy,
            &mut writer,
            seg_idx,
            |msg| {
                if let Some(data) = msg.strip_prefix("TOTAL:") {
                    seg_total.set(data.parse().unwrap_or(0));
                } else if let Some(data) = msg.strip_prefix("PHASE1:") {
                    let parts: Vec<&str> = data.split('/').collect();
                    let scanned: usize = parts.get(0).and_then(|s| s.parse().ok()).unwrap_or(0);
                    let total: usize = parts
                        .get(1)
                        .and_then(|s| s.split_once(" skipped:").map(|(n, _)| n))
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                    let skipped: usize = data
                        .split_once("skipped:")
                        .and_then(|(_, s)| s.parse().ok())
                        .unwrap_or(0);
                    let pct = if total > 0 {
                        (scanned * 100) / total
                    } else {
                        0
                    };
                    pb.set_message(format!(
                        "Segment {}/{} [scanning {}/{} skipped:{} {}%]",
                        seg_idx + 1,
                        total_segments,
                        scanned,
                        total,
                        skipped,
                        pct,
                    ));
                } else if let Some(data) = msg.strip_prefix("PROGRESS:") {
                    let parts: Vec<&str> = data.split(':').collect();
                    let migrated: usize = parts.get(0).and_then(|s| s.parse().ok()).unwrap_or(0);
                    let total = seg_total.get();
                    let pct = if total > 0 {
                        (migrated * 100) / total
                    } else {
                        0
                    };
                    pb.set_message(format!(
                        "Segment {}/{} [migrating {}/{} {}%]",
                        seg_idx + 1,
                        total_segments,
                        migrated,
                        total,
                        pct,
                    ));
                } else if let Some(warn) = msg.strip_prefix("WARN:") {
                    pb.println(format!("{} {}", style("⚠").yellow(), warn));
                } else if let Some(done_data) = msg.strip_prefix("DONE:") {
                    let parts: Vec<&str> = done_data.split(':').collect();
                    let migrated: usize = parts.get(0).and_then(|s| s.parse().ok()).unwrap_or(0);
                    let skipped: usize = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                    grand_total_migrated += migrated;
                    grand_total_skipped += skipped;
                }
            },
        ) {
            Ok(()) => {}
            Err(e) => {
                pb.finish_with_message(format!("{}", style("Migration failed.").red()));
                eprintln!("\n{} {:?}", style("✘").red().bold(), e);
                return;
            }
        }

        pb.set_position((seg_idx + 1) as u64);
    }

    pb.set_message(style("Finalizing indexes...").dim().to_string());
    if let Err(e) = writer.finish_writers() {
        pb.finish_with_message(format!("{}", style("Migration failed.").red()));
        eprintln!("\n{} {:?}", style("✘").red().bold(), e);
        return;
    }

    pb.set_message(style("Shutting down blob engine...").dim().to_string());
    if let Err(e) = writer.shutdown_engine() {
        pb.finish_with_message(format!("{}", style("Migration failed.").red()));
        eprintln!("\n{} {:?}", style("✘").red().bold(), e);
        return;
    }

    // Write STORAGE_VERSION = 2 to mark the data as v2.x compatible
    if let Err(e) = write_storage_version(&root_path, 2) {
        pb.finish_with_message(format!("{}", style("Migration failed.").red()));
        eprintln!(
            "\n{} Failed to write STORAGE_VERSION: {:?}",
            style("✘").red().bold(),
            e
        );
        return;
    }

    pb.finish_with_message(format!(
        "Migration finished. Total: {}, Skipped: {}",
        grand_total_migrated, grand_total_skipped
    ));

    println!(
        "{} {}",
        style("✔").green(),
        style("Migration to v2.x completed successfully!").bold()
    );
}

/// Migrate all documents from a single EML segment to the v2.x storage layout.
fn do_migrate_segment_v2<F>(
    batch_size: u32,
    legacy: LegacyDirs,
    writer: &mut NewIndexWriterV2,
    segment_index: usize,
    mut on_progress: F,
) -> MailboxdResult<()>
where
    F: FnMut(&str),
{
    // ── open legacy indices ────────────────────────────────────────────
    let envelope_index = Index::open_in_dir(&legacy.envelope_dir)
        .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;
    let eml_index = Index::open_in_dir(&legacy.eml_dir)
        .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;

    let envelope_reader = envelope_index
        .reader()
        .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;
    let eml_reader = eml_index
        .reader()
        .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;

    let envelope_searcher = envelope_reader.searcher();
    let eml_searcher = eml_reader.searcher();

    let ef = SchemaTools::envelope_fields();
    let mf = SchemaTools::eml_fields();

    let eml_segments = eml_searcher.segment_readers();
    let eml_segment = eml_segments.get(segment_index).ok_or_else(|| {
        raise_error!(
            format!(
                "segment index {} out of range ({} segments)",
                segment_index,
                eml_segments.len()
            ),
            ErrorCode::InternalError
        )
    })?;

    let num_docs = eml_segment.num_docs();
    if num_docs == 0 {
        on_progress("TOTAL:0");
        on_progress("DONE:0:0");
        return Ok(());
    }

    on_progress(&format!("TOTAL:{}", num_docs));

    let max_doc = eml_segment.max_doc();
    let ff = eml_segment.fast_fields();
    let f_id_col: Column<u64> = ff.u64("id").map_err(|e| {
        raise_error!(
            format!("failed to open f_id fast field: {e:#?}"),
            ErrorCode::InternalError
        )
    })?;

    // ── Phase 1: build eid → (uid, internal_date) from envelope, then drop it ──
    let mut envelope_map: HashMap<u64, (u32, i64)> = HashMap::with_capacity(num_docs as usize);

    let mut env_scanned = 0u32;
    let mut env_skipped = 0u32;
    for doc_id in 0..max_doc {
        if eml_segment.is_deleted(doc_id) {
            continue;
        }
        let eid = f_id_col.values.get_val(doc_id);

        let term = Term::from_field_u64(ef.f_id, eid);
        let query = TermQuery::new(term, IndexRecordOption::Basic);
        let hits: Vec<(_, DocAddress)> = envelope_searcher
            .search(&query, &TopDocs::with_limit(1).order_by_score())
            .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;

        if let Some((_, addr)) = hits.first() {
            let env_doc: TantivyDocument = envelope_searcher
                .doc(*addr)
                .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;
            let uid = env_doc
                .get_first(ef.f_uid)
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32;
            let internal_date = env_doc
                .get_first(ef.f_internal_date)
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            envelope_map.insert(eid, (uid, internal_date));
            env_scanned += 1;
        } else {
            env_skipped += 1;
        }

        if env_scanned % 10 == 0 {
            on_progress(&format!(
                "PHASE1:{}/{} skipped:{}",
                env_scanned, max_doc, env_skipped
            ));
        }
    }

    // Free the envelope index before the heavy EML processing.
    drop(envelope_searcher);
    drop(envelope_reader);
    drop(envelope_index);

    // ── Phase 2: process EML docs, streaming one at a time ─────────────
    let mut total_migrated = 0usize;
    let mut total_skipped = 0usize;

    let mut chunk_start = 0u32;

    while chunk_start < max_doc {
        let chunk_end = (chunk_start + batch_size).min(max_doc);
        let store_reader = eml_segment
            .get_store_reader(2)
            .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;

        for doc_id in chunk_start..chunk_end {
            if eml_segment.is_deleted(doc_id) {
                continue;
            }

            let eid = f_id_col.values.get_val(doc_id);

            let (uid, internal_date) = match envelope_map.get(&eid) {
                Some(v) => *v,
                None => {
                    on_progress(&format!("WARN: eid {} envelope not found", eid));
                    total_skipped += 1;
                    continue;
                }
            };

            let eml_doc: TantivyDocument = store_reader
                .get(doc_id)
                .map_err(|e| raise_error!(format!("{e:#?}"), ErrorCode::InternalError))?;

            let account_id = match eml_doc.get_first(mf.f_account_id).and_then(|v| v.as_u64()) {
                Some(v) => v,
                None => {
                    on_progress(&format!("WARN: eid {} account_id missing", eid));
                    total_skipped += 1;
                    continue;
                }
            };
            let mailbox_id = eml_doc
                .get_first(mf.f_mailbox_id)
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            let eml_bytes = match eml_doc.get_first(mf.f_eml).and_then(|v| v.as_bytes()) {
                Some(b) => b,
                None => {
                    on_progress(&format!("WARN: eid {} eml bytes missing", eid));
                    total_skipped += 1;
                    continue;
                }
            };

            if let Err(e) = writer.ingest(eml_bytes, account_id, mailbox_id, uid, internal_date) {
                on_progress(&format!(
                    "ERROR: Account {} eid {} ingest failed: {}",
                    account_id, eid, e
                ));
                total_skipped += 1;
                continue;
            }

            total_migrated += 1;

            if total_migrated % 10 == 0 || total_migrated as u32 == num_docs {
                on_progress(&format!("PROGRESS:{}:{}", total_migrated, num_docs));
            }
        }

        drop(store_reader);

        // Flush blob buffers to mailboxd-blob engine.
        writer.flush_blob_buffers()?;

        chunk_start = chunk_end;
    }

    on_progress(&format!("DONE:{}:{}", total_migrated, total_skipped));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fabricate a directory that `is_tantivy_index_dir` accepts as a Tantivy
    /// index: a `meta.json` plus at least three files with Tantivy extensions.
    /// Contents are irrelevant — only the file names are inspected.
    fn fake_tantivy_dir(path: &Path) {
        std::fs::create_dir_all(path).unwrap();
        std::fs::write(path.join("meta.json"), b"{}").unwrap();
        std::fs::write(path.join("00000000000000000000000000000000.store"), b"").unwrap();
        std::fs::write(path.join("00000000000000000000000000000000.term"), b"").unwrap();
        std::fs::write(path.join("00000000000000000000000000000000.idx"), b"").unwrap();
    }

    // ── V037Paths::derive ─────────────────────────────────────────────

    #[test]
    fn derive_uses_root_defaults_when_no_env_dirs() {
        let root = PathBuf::from("/data");
        let paths = V037Paths::derive(&root, None, None);
        assert_eq!(paths.root, root);
        assert_eq!(paths.legacy_index, root.join("envelope"));
        assert_eq!(paths.legacy_data, root.join("eml"));
        assert_eq!(paths.new_index, root.join("mailboxd-indices"));
        assert_eq!(paths.new_data, root.join("mailboxd-storage"));
    }

    #[test]
    fn derive_honours_explicit_index_and_data_dirs() {
        let root = PathBuf::from("/data");
        let index_dir = PathBuf::from("/idx");
        let data_dir = PathBuf::from("/store");
        let paths = V037Paths::derive(&root, Some(&index_dir), Some(&data_dir));
        // The legacy Tantivy indices sit at the given dirs directly …
        assert_eq!(paths.legacy_index, index_dir);
        assert_eq!(paths.legacy_data, data_dir);
        // … and the new layout is written beneath them, matching the
        // interactive tool and the runtime's directory manager.
        assert_eq!(paths.new_index, index_dir.join("mailboxd-indices"));
        assert_eq!(paths.new_data, data_dir.join("mailboxd-storage"));
    }

    // ── dir_has_entries ───────────────────────────────────────────────

    #[test]
    fn dir_has_entries_detects_content_and_absence() {
        let tmp = tempfile::tempdir().unwrap();
        let empty = tmp.path().join("empty");
        std::fs::create_dir_all(&empty).unwrap();
        assert!(!dir_has_entries(&empty).unwrap());
        assert!(!dir_has_entries(&tmp.path().join("missing")).unwrap());

        std::fs::write(empty.join("x"), b"y").unwrap();
        assert!(dir_has_entries(&empty).unwrap());
    }

    // ── migrate_v037_to_v2 guards ─────────────────────────────────────

    #[test]
    fn migrate_v037_is_noop_when_already_v2() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        write_storage_version(&root, CURRENT_STORAGE_VERSION).unwrap();
        // A legacy layout present alongside a v2 marker must not tempt a
        // re-migration.
        fake_tantivy_dir(&root.join("envelope"));
        fake_tantivy_dir(&root.join("eml"));

        let paths = V037Paths::derive(&root, None, None);
        assert_eq!(migrate_v037_to_v2(&paths, 100).unwrap(), None);
    }

    #[test]
    fn migrate_v037_is_noop_when_no_legacy_layout() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        // Fresh volume: no envelope/eml Tantivy indices.
        let paths = V037Paths::derive(&root, None, None);
        assert_eq!(migrate_v037_to_v2(&paths, 100).unwrap(), None);
    }

    #[test]
    fn migrate_v037_refuses_when_v2_target_already_exists() {
        // A legacy layout is present, but a `blobs/` directory already exists
        // and no STORAGE_VERSION marker was written — the signature of an
        // interrupted migration. Auto-migration must refuse rather than
        // re-ingest and double the Tantivy documents.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        fake_tantivy_dir(&root.join("envelope"));
        fake_tantivy_dir(&root.join("eml"));
        std::fs::create_dir_all(root.join("mailboxd-storage").join("blobs")).unwrap();

        let paths = V037Paths::derive(&root, None, None);
        let err = migrate_v037_to_v2(&paths, 100).unwrap_err();
        assert!(
            err.to_string().contains("interrupted migration"),
            "unexpected error: {err}"
        );
    }

    // ── full v0.3.7 → v2 happy path ───────────────────────────────────

    const SAMPLE_EML: &[u8] = b"From: alice@example.com\r\n\
To: bob@example.com\r\n\
Subject: Legacy migration test\r\n\
Date: Mon, 1 Jan 2024 00:00:00 +0000\r\n\
Message-ID: <legacy-1@example.com>\r\n\
\r\n\
Body of a v0.3.7 archived message.\r\n";

    /// Build a realistic legacy v0.3.7 volume at `root`: empty native_db
    /// `meta.db`/`mailbox.db` (so the metadata pre-flight passes) plus a
    /// legacy `eml` and `envelope` Tantivy index each holding one message.
    /// Returns the content hash the migration must key the email blob under.
    fn seed_v037_volume(root: &Path) -> String {
        use crate::legacy::schema::SchemaTools;
        use mailboxd_core::utils::compute_content_hash;

        // Empty legacy metadata databases (native_db / redb files). Dropped
        // immediately so the migration can reopen them.
        {
            let db = native_db::Builder::new()
                .create(&crate::meta::META_MODELS, root.join("meta.db"))
                .unwrap();
            drop(db);
            let db = native_db::Builder::new()
                .create(&crate::meta::MAILBOX_MODELS, root.join("mailbox.db"))
                .unwrap();
            drop(db);
        }

        let eid: u64 = 42;
        let account_id: u64 = 7;
        let mailbox_id: u64 = 3;

        // Legacy EML index (raw message bytes live in Tantivy here).
        {
            let ef = SchemaTools::eml_fields();
            std::fs::create_dir_all(root.join("eml")).unwrap();
            let index =
                tantivy::Index::create_in_dir(root.join("eml"), SchemaTools::eml_schema()).unwrap();
            let mut w = index.writer_with_num_threads(1, 50_000_000).unwrap();
            let mut d = tantivy::TantivyDocument::default();
            d.add_field_value(ef.f_id, &eid);
            d.add_field_value(ef.f_account_id, &account_id);
            d.add_field_value(ef.f_mailbox_id, &mailbox_id);
            d.add_field_value(ef.f_eml, &SAMPLE_EML.to_vec());
            w.add_document(d).unwrap();
            w.commit().unwrap();
        }

        // Legacy envelope index (uid + internal_date keyed by the same eid).
        {
            let ev = SchemaTools::envelope_fields();
            std::fs::create_dir_all(root.join("envelope")).unwrap();
            let index =
                tantivy::Index::create_in_dir(root.join("envelope"), SchemaTools::envelope_schema())
                    .unwrap();
            let mut w = index.writer_with_num_threads(1, 50_000_000).unwrap();
            let mut d = tantivy::TantivyDocument::default();
            d.add_field_value(ev.f_id, &eid);
            d.add_field_value(ev.f_uid, &100u64);
            d.add_field_value(ev.f_internal_date, &1_700_000_000_000i64);
            w.add_document(d).unwrap();
            w.commit().unwrap();
        }

        // With no attachments the stored email blob is the original bytes,
        // keyed by their content hash.
        compute_content_hash(SAMPLE_EML)
    }

    #[test]
    fn migrate_v037_converts_legacy_volume_end_to_end() {
        use mailboxd_blob::{Config, Engine};

        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let hex_hash = seed_v037_volume(&root);

        // Broken pre-conditions of an existing v0.3.7 install.
        assert!(read_storage_version(&root).is_none());
        assert!(!root.join("mailboxd-storage").join("blobs").exists());

        let paths = V037Paths::derive(&root, None, None);
        let outcome = migrate_v037_to_v2(&paths, 100)
            .unwrap()
            .expect("legacy v0.3.7 volume should migrate");
        assert_eq!(outcome.migrated, 1);
        assert_eq!(outcome.skipped, 0);

        // The server's boot gate now passes …
        assert_eq!(read_storage_version(&root), Some(CURRENT_STORAGE_VERSION));
        // … the v2 artifacts exist …
        assert!(root.join("memdb").is_dir());
        assert!(root.join("mailboxd-indices").join("mail_metadata").is_dir());

        // … and the archived message survives byte-for-byte in the new engine.
        let mut config = Config::default();
        config.flush_interval_secs = 0;
        config.gc_interval_secs = 0;
        let engine = Engine::open(&root.join("mailboxd-storage").join("blobs"), config).unwrap();
        let mut key = [0u8; 32];
        hex::decode_to_slice(&hex_hash, &mut key).unwrap();
        assert_eq!(
            engine.get(&key).unwrap().as_deref(),
            Some(SAMPLE_EML),
            "migrated email blob must be byte-identical"
        );
        engine.shutdown().unwrap();

        // Re-running now short-circuits on the STORAGE_VERSION marker.
        assert_eq!(migrate_v037_to_v2(&paths, 100).unwrap(), None);
    }
}

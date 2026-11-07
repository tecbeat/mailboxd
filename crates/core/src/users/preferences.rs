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

//! Web UI preferences kept on the user profile, so they follow the user
//! across browsers and private windows.
//!
//! Hidden table columns are stored per table as the set of hidden column
//! ids. A user who never changes anything sees every column, and columns
//! added later show up by default.
//!
//! The column order is stored per table as a list of column ids. The Web UI
//! appends columns missing from the list in their default order, so a
//! column added later shows up at its default position relative to the end.
//!
//! Date patterns are Moment.js-style format strings the Web UI interprets;
//! the server only bounds their size.

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    error::{code::ErrorCode, MailboxdResult},
    raise_error,
};

pub type HiddenColumns = BTreeMap<String, BTreeSet<String>>;

pub type ColumnOrder = BTreeMap<String, Vec<String>>;

/// The tables whose columns can be hidden, with the columns that may be
/// hidden. Columns not listed here are always shown.
const HIDEABLE_COLUMNS: &[(&str, &[&str])] = &[(
    "accounts",
    &[
        "id",
        "account_name",
        "account_type",
        "sync_interval_sec",
        "running_state",
        "created_by",
        "created_at",
        "updated_at",
    ],
)];

/// The tables whose columns can be reordered, with the columns that may be
/// moved. The actions column always stays last.
const ORDERABLE_COLUMNS: &[(&str, &[&str])] = &[(
    "accounts",
    &[
        "id",
        "account_name",
        "email",
        "enabled",
        "auth_type",
        "account_type",
        "sync_interval_sec",
        "running_state",
        "created_by",
        "created_at",
        "updated_at",
    ],
)];

fn table_columns(
    tables: &[(&str, &'static [&'static str])],
    table: &str,
) -> Option<&'static [&'static str]> {
    tables
        .iter()
        .find(|(name, _)| *name == table)
        .map(|(_, columns)| *columns)
}

fn hideable_columns(table: &str) -> Option<&'static [&'static str]> {
    table_columns(HIDEABLE_COLUMNS, table)
}

fn orderable_columns(table: &str) -> Option<&'static [&'static str]> {
    table_columns(ORDERABLE_COLUMNS, table)
}

pub fn validate_hidden_columns(hidden: &HiddenColumns) -> MailboxdResult<()> {
    for (table, columns) in hidden {
        let Some(allowed) = hideable_columns(table) else {
            return Err(raise_error!(
                format!("unknown table '{table}' in hidden_columns"),
                ErrorCode::InvalidParameter
            ));
        };
        if let Some(column) = columns.iter().find(|c| !allowed.contains(&c.as_str())) {
            return Err(raise_error!(
                format!("column '{column}' of table '{table}' cannot be hidden"),
                ErrorCode::InvalidParameter
            ));
        }
    }
    Ok(())
}

pub fn validate_column_order(order: &ColumnOrder) -> MailboxdResult<()> {
    for (table, columns) in order {
        let Some(allowed) = orderable_columns(table) else {
            return Err(raise_error!(
                format!("unknown table '{table}' in column_order"),
                ErrorCode::InvalidParameter
            ));
        };
        let mut seen = BTreeSet::new();
        for column in columns {
            if !allowed.contains(&column.as_str()) {
                return Err(raise_error!(
                    format!("column '{column}' of table '{table}' cannot be moved"),
                    ErrorCode::InvalidParameter
                ));
            }
            if !seen.insert(column.as_str()) {
                return Err(raise_error!(
                    format!("column '{column}' appears twice in the order of table '{table}'"),
                    ErrorCode::InvalidParameter
                ));
            }
        }
    }
    Ok(())
}

/// Longest date pattern accepted, in characters; matches the Web UI.
const MAX_DATE_PATTERN_CHARS: usize = 64;

pub fn validate_date_pattern(pattern: &str) -> MailboxdResult<()> {
    if pattern.chars().count() > MAX_DATE_PATTERN_CHARS {
        return Err(raise_error!(
            format!("date pattern must be at most {MAX_DATE_PATTERN_CHARS} characters"),
            ErrorCode::InvalidParameter
        ));
    }
    if pattern.chars().any(char::is_control) {
        return Err(raise_error!(
            "date pattern must not contain control characters".into(),
            ErrorCode::InvalidParameter
        ));
    }
    Ok(())
}

/// The stored form of a date pattern sent by the Web UI: trimmed, and
/// `None` (language default) when empty.
pub fn normalize_date_pattern(pattern: &str) -> Option<String> {
    let trimmed = pattern.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// Replace the hidden columns of every table in `update`; other tables keep
/// theirs. A table with no hidden columns left is dropped.
pub fn merge_hidden_columns(current: &mut HiddenColumns, update: HiddenColumns) {
    replace_tables(current, update, BTreeSet::is_empty);
}

/// Replace the column order of every table in `update`; other tables keep
/// theirs. A table sent with an empty order goes back to the default order.
pub fn merge_column_order(current: &mut ColumnOrder, update: ColumnOrder) {
    replace_tables(current, update, Vec::is_empty);
}

fn replace_tables<C>(
    current: &mut BTreeMap<String, C>,
    update: BTreeMap<String, C>,
    is_empty: fn(&C) -> bool,
) {
    for (table, columns) in update {
        if is_empty(&columns) {
            current.remove(&table);
        } else {
            current.insert(table, columns);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hidden(entries: &[(&str, &[&str])]) -> HiddenColumns {
        entries
            .iter()
            .map(|(table, columns)| {
                (
                    table.to_string(),
                    columns.iter().map(|c| c.to_string()).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn every_optional_accounts_column_can_be_hidden() {
        let all = hideable_columns("accounts").unwrap();
        assert!(validate_hidden_columns(&hidden(&[("accounts", all)])).is_ok());
    }

    #[test]
    fn email_auth_enabled_and_actions_cannot_be_hidden() {
        for column in ["email", "auth_type", "enabled", "actions"] {
            assert!(
                validate_hidden_columns(&hidden(&[("accounts", &[column])])).is_err(),
                "{column} must stay visible"
            );
        }
    }

    #[test]
    fn unknown_tables_are_rejected() {
        assert!(validate_hidden_columns(&hidden(&[("users", &["id"])])).is_err());
    }

    #[test]
    fn an_empty_selection_is_valid() {
        assert!(validate_hidden_columns(&HiddenColumns::new()).is_ok());
        assert!(validate_hidden_columns(&hidden(&[("accounts", &[])])).is_ok());
    }

    #[test]
    fn merging_replaces_only_the_tables_in_the_update() {
        let mut current = hidden(&[("accounts", &["id", "account_name"]), ("other", &["x"])]);
        merge_hidden_columns(&mut current, hidden(&[("accounts", &["created_at"])]));
        assert_eq!(
            current,
            hidden(&[("accounts", &["created_at"]), ("other", &["x"])])
        );
    }

    fn order(entries: &[(&str, &[&str])]) -> ColumnOrder {
        entries
            .iter()
            .map(|(table, columns)| {
                (
                    table.to_string(),
                    columns.iter().map(|c| c.to_string()).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn every_accounts_column_but_actions_can_be_moved() {
        let mut all = orderable_columns("accounts").unwrap().to_vec();
        all.reverse();
        assert!(validate_column_order(&order(&[("accounts", &all)])).is_ok());
        assert!(validate_column_order(&order(&[("accounts", &["email", "actions"])])).is_err());
    }

    #[test]
    fn a_partial_column_order_is_valid() {
        assert!(validate_column_order(&order(&[("accounts", &["updated_at", "id"])])).is_ok());
        assert!(validate_column_order(&order(&[("accounts", &[])])).is_ok());
    }

    #[test]
    fn a_column_order_with_duplicates_or_unknown_ids_is_rejected() {
        assert!(validate_column_order(&order(&[("accounts", &["id", "email", "id"])])).is_err());
        assert!(validate_column_order(&order(&[("accounts", &["password"])])).is_err());
        assert!(validate_column_order(&order(&[("users", &["id"])])).is_err());
    }

    #[test]
    fn merging_a_column_order_replaces_only_the_tables_sent() {
        let mut current = order(&[("accounts", &["email", "id"]), ("other", &["x"])]);
        merge_column_order(&mut current, order(&[("accounts", &["id", "email"])]));
        assert_eq!(
            current,
            order(&[("accounts", &["id", "email"]), ("other", &["x"])])
        );
        merge_column_order(&mut current, order(&[("accounts", &[])]));
        assert_eq!(current, order(&[("other", &["x"])]));
    }

    #[test]
    fn date_patterns_up_to_64_characters_are_valid() {
        assert!(validate_date_pattern("").is_ok());
        assert!(validate_date_pattern("DD.MM.YYYY [um] HH:mm").is_ok());
        assert!(validate_date_pattern(&"Ä".repeat(64)).is_ok());
        assert!(validate_date_pattern(&"D".repeat(65)).is_err());
    }

    #[test]
    fn date_patterns_with_control_characters_are_rejected() {
        assert!(validate_date_pattern("DD\nMM").is_err());
        assert!(validate_date_pattern("DD\u{0}").is_err());
    }

    #[test]
    fn an_empty_date_pattern_means_the_language_default() {
        assert_eq!(normalize_date_pattern("  "), None);
        assert_eq!(
            normalize_date_pattern(" YYYY-MM-DD "),
            Some("YYYY-MM-DD".into())
        );
    }

    #[test]
    fn merging_an_empty_selection_shows_every_column_again() {
        let mut current = hidden(&[("accounts", &["id"])]);
        merge_hidden_columns(&mut current, hidden(&[("accounts", &[])]));
        assert!(current.is_empty());
    }
}

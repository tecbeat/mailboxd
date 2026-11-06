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

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    error::{code::ErrorCode, MailboxdResult},
    raise_error,
};

pub type HiddenColumns = BTreeMap<String, BTreeSet<String>>;

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

fn hideable_columns(table: &str) -> Option<&'static [&'static str]> {
    HIDEABLE_COLUMNS
        .iter()
        .find(|(name, _)| *name == table)
        .map(|(_, columns)| *columns)
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

/// Replace the hidden columns of every table in `update`; other tables keep
/// theirs. A table with no hidden columns left is dropped.
pub fn merge_hidden_columns(current: &mut HiddenColumns, update: HiddenColumns) {
    for (table, columns) in update {
        if columns.is_empty() {
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

    #[test]
    fn merging_an_empty_selection_shows_every_column_again() {
        let mut current = hidden(&[("accounts", &["id"])]);
        merge_hidden_columns(&mut current, hidden(&[("accounts", &[])]));
        assert!(current.is_empty());
    }
}

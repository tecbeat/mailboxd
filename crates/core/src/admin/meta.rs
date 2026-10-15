//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
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

use std::path::Path;

use mailboxd_memdb::{Durability, MemDb};

use crate::{
    database::MemDbModel,
    error::{code::ErrorCode, MailboxdResult},
    raise_error,
    users::{hash_login_password, UserModel, DEFAULT_ADMIN_USER_ID},
};

pub fn open_database(path: impl AsRef<Path>) -> MailboxdResult<MemDb> {
    MemDb::open_with(path, Durability::Full).map_err(|e| {
        raise_error!(
            format!("Failed to open database: {:?}", e),
            ErrorCode::InternalError
        )
    })
}

pub fn find_admin(db: &MemDb) -> MailboxdResult<Option<UserModel>> {
    let key = DEFAULT_ADMIN_USER_ID.to_string();
    let coll = db.collection(UserModel::collection());
    coll.get(&key)
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))
}

pub fn update_admin_password(db: &MemDb, password: String) -> MailboxdResult<()> {
    let key = DEFAULT_ADMIN_USER_ID.to_string();
    let coll = db.collection(UserModel::collection());
    let entity: UserModel = coll
        .get_required(&key)
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

    let mut updated = entity.clone();
    updated.password = Some(hash_login_password(&password)?);

    coll.upsert(&key, &updated)
        .map_err(|e| raise_error!(format!("{:#?}", e), ErrorCode::InternalError))?;

    Ok(())
}

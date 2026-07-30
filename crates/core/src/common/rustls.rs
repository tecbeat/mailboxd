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

use crate::{
    {
        context::Initialize,
        error::{code::ErrorCode, MailboxdResult},
    },
    raise_error,
};

pub struct MailboxdTls;

impl Initialize for MailboxdTls {
    async fn initialize() -> MailboxdResult<()> {
        rustls::crypto::CryptoProvider::install_default(rustls::crypto::ring::default_provider())
            .map_err(|_| {
                raise_error!(
                    "failed to set crypto provider".into(),
                    ErrorCode::InternalError
                )
            })
    }
}

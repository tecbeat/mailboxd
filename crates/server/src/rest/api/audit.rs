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

use crate::common::auth::WrappedContext;
use crate::rest::api::ApiTags;
use crate::rest::ApiResult;
use mailboxd_core::audit::{self, AuditEntry};
use mailboxd_core::common::paginated::DataPage;
use mailboxd_core::users::permissions::Permission;
use poem_openapi::param::Query;
use poem_openapi::payload::Json;
use poem_openapi::OpenApi;

pub struct AuditApi;

#[OpenApi(prefix_path = "/api/v1", tag = "ApiTags::Audit")]
impl AuditApi {
    /// List security-relevant audit-log entries, newest first.
    ///
    /// Read-only and restricted to administrators. Omitting `page` and
    /// `page_size` returns all entries in a single page.
    #[oai(method = "get", path = "/audit-log", operation_id = "list_audit_log")]
    async fn list_audit_log(
        &self,
        /// The page number for pagination (1-based).
        page: Query<Option<u64>>,
        /// The number of entries per page.
        page_size: Query<Option<u64>>,
        context: WrappedContext,
    ) -> ApiResult<Json<DataPage<AuditEntry>>> {
        context.require_permission(None, Permission::ROOT)?;
        let paginated = audit::list(page.0, page_size.0)?;
        Ok(Json(paginated.into()))
    }
}

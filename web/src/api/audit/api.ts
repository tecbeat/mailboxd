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


import axiosInstance from "@/api/axiosInstance";
import { PaginatedResponse } from "..";

export interface AuditEntry {
  id: string;
  created_at: number;
  event_type: string;
  actor: string;
  target: string | null;
  ip: string | null;
  detail: string | null;
  success: boolean;
}

export const list_audit_log = async (page: number, page_size: number) => {
  const response = await axiosInstance.get<PaginatedResponse<AuditEntry>>(
    "api/v1/audit-log",
    { params: { page, page_size } },
  );
  return response.data;
};

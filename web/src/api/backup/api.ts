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

import { saveAs } from "file-saver";
import axiosInstance from "@/api/axiosInstance";

export interface BackupConfig {
  scheduled_enabled: boolean;
  backup_dir: string | null;
  schedule: string | null;
  retention: number;
  last_backup_at: string | null;
  scheduled_count: number;
}

export interface RestoreStagedResponse {
  staged: boolean;
  message: string;
}

/** Fetch the current backup schedule configuration and status. */
export const get_backup_config = async () => {
  const response = await axiosInstance.get<BackupConfig>("api/v1/backup-config");
  return response.data;
};

/**
 * Parse the filename from a Content-Disposition header, falling back to a
 * generated name when the server does not provide one.
 */
const filenameFromDisposition = (disposition: unknown): string => {
  const fallback = `mailboxd-backup-${new Date()
    .toISOString()
    .replace(/[:.]/g, "-")}.tar.zst`;
  if (typeof disposition !== "string") return fallback;
  const match = /filename\*?=(?:UTF-8'')?"?([^";]+)"?/i.exec(disposition);
  return match ? decodeURIComponent(match[1]) : fallback;
};

/**
 * Create a full backup on the server and download it as a .tar.zst file.
 *
 * The download can take a while for large archives, so the per-request
 * timeout is disabled.
 */
export const create_backup = async () => {
  const response = await axiosInstance.post("api/v1/create-backup", undefined, {
    responseType: "blob",
    timeout: 0,
  });
  const filename = filenameFromDisposition(
    response.headers["content-disposition"],
  );
  saveAs(new Blob([response.data]), filename);
};

/**
 * Upload a backup archive and stage it for restore-on-restart.
 *
 * The server validates and stages the archive, then restarts to apply it.
 * The per-request timeout is disabled to allow large uploads.
 */
export const restore_backup = async (
  file: File,
  onProgress?: (pct: number) => void,
): Promise<RestoreStagedResponse> => {
  const response = await axiosInstance.post<RestoreStagedResponse>(
    "api/v1/restore-backup",
    file,
    {
      headers: { "Content-Type": "application/octet-stream" },
      timeout: 0,
      onUploadProgress: (e) => {
        if (e.total && onProgress) {
          onProgress(Math.round((e.loaded / e.total) * 100));
        }
      },
    },
  );
  return response.data;
};

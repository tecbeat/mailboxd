//
// Copyright (c) 2026 tecbeat
//
// This file is part of mailboxd, a fork of the Bichon email archiving
// project.
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

export type FolderMode = '' | 'header' | 'existing' | 'custom';

export interface FolderSelection {
  /** A PST file is selected. */
  isPst: boolean;
  folderMode: FolderMode;
  /** Folder detected from the message headers (or the `INBOX` default). */
  headerFolder: string;
  /** Folder picked from the existing list or typed in. */
  folder: string;
}

/**
 * Folder the upload is sent to. PST files carry their own folder structure
 * and have no folder mode, but the upload endpoint still requires a
 * non-empty folder, so they use the header folder.
 */
export function resolveEffectiveFolder({ isPst, folderMode, headerFolder, folder }: FolderSelection): string {
  if (isPst) return headerFolder;
  switch (folderMode) {
    case 'header':
      return headerFolder;
    case 'existing':
    case 'custom':
      return folder;
    default:
      return '';
  }
}

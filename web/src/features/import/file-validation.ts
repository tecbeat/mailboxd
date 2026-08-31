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

/**
 * Pure validation logic for the import file queue.
 *
 * Kept free of React so the size/type rules can be unit-tested in isolation
 * and so callers must pass the *current* upload limits explicitly — the limits
 * are parameters, never captured from a closure.
 */

/** Hardcoded EML size limit: 100 MB. */
export const MAX_EML = 100 * 1024 * 1024;

/** MIME types that are clearly NOT email files — reject these upfront. */
const BLOCKED_MIME_PREFIXES = [
  'video/', 'audio/', 'image/', 'font/',
  'application/zip', 'application/gzip', 'application/x-tar',
  'application/x-7z', 'application/x-rar',
  'application/vnd.', 'application/pdf',
  'application/x-msdownload', 'application/x-executable',
];

/** Server-configured upload limits, in bytes. */
export interface UploadLimits {
  maxMbox: number;
  maxPst: number;
}

/** A file queued for import, tagged with the outcome of validation. */
export interface QueuedFile {
  file: File;
  sizeOk: boolean;
  typeOk: boolean;
}

/** Lower-cased file extension without the dot (e.g. `"mbox"`), `""` when absent. */
export function getExtension(fileName: string): string {
  return fileName.split('.').pop()?.toLowerCase() || '';
}

/** True when the file looks like an email file: an allowed extension and no blocked MIME type. */
export function isValidFileType(file: File, ext: string): boolean {
  const mime = file.type.toLowerCase();
  if (mime) {
    for (const prefix of BLOCKED_MIME_PREFIXES) {
      if (mime.startsWith(prefix)) return false;
    }
  }
  return ext === 'eml' || ext === 'mbox' || ext === 'pst';
}

/** Byte limit that applies to a given extension. */
function limitForExt(ext: string, limits: UploadLimits): number {
  if (ext === 'mbox') return limits.maxMbox;
  if (ext === 'pst') return limits.maxPst;
  return MAX_EML;
}

/** Classify a single file against the current upload limits. */
export function classifyFile(file: File, limits: UploadLimits): QueuedFile {
  const ext = getExtension(file.name);
  return {
    file,
    sizeOk: file.size <= limitForExt(ext, limits),
    typeOk: isValidFileType(file, ext),
  };
}

/** Classify a batch of files against the current upload limits, preserving order. */
export function buildQueue(files: FileList | File[], limits: UploadLimits): QueuedFile[] {
  return (Array.from(files) as File[]).map((f) => classifyFile(f, limits));
}

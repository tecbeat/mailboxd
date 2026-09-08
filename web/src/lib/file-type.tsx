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

import {
  type LucideIcon,
  FileText,
  FileImage,
  Video as FileVideo,
  FileMusic as FileAudio,
  FileArchive,
  FileCode,
  FilePlus,
  FileLock,
  Presentation,
  FileSpreadsheet,
} from 'lucide-react';

export interface FileTypeConfig {
  Icon: LucideIcon;
  text: string;
  badge: string;
}

export function getFileTypeConfig(mimeType: string): FileTypeConfig {
  const type = mimeType.toLowerCase();

  if (type.includes('pdf')) {
    return { Icon: FileText, text: 'text-red-600', badge: 'text-red-600 bg-red-50 border-red-100' };
  }
  if (type.includes('word') || type.includes('officedocument.word') || type === 'application/msword') {
    return { Icon: FileText, text: 'text-blue-600', badge: 'text-blue-600 bg-blue-50 border-blue-100' };
  }
  if (type.includes('presentation') || type.includes('powerpoint')) {
    return { Icon: Presentation, text: 'text-orange-600', badge: 'text-orange-600 bg-orange-50 border-orange-100' };
  }
  if (type.includes('spreadsheet') || type.includes('excel') || type.includes('csv')) {
    return { Icon: FileSpreadsheet, text: 'text-green-600', badge: 'text-green-600 bg-green-50 border-green-100' };
  }
  if (type.startsWith('image/')) {
    return { Icon: FileImage, text: 'text-purple-600', badge: 'text-purple-600 bg-purple-50 border-purple-100' };
  }
  if (type.startsWith('video/')) {
    return { Icon: FileVideo, text: 'text-pink-600', badge: 'text-pink-600 bg-pink-50 border-pink-100' };
  }
  if (type.startsWith('audio/')) {
    return { Icon: FileAudio, text: 'text-amber-600', badge: 'text-amber-600 bg-amber-50 border-amber-100' };
  }
  if (
    type.includes('zip') ||
    type.includes('tar') ||
    type.includes('rar') ||
    type.includes('7z') ||
    type.includes('compressed') ||
    type.includes('archive')
  ) {
    return { Icon: FileArchive, text: 'text-gray-600', badge: 'text-gray-600 bg-gray-50 border-gray-100' };
  }
  if (type.startsWith('text/') || type.includes('json') || type.includes('javascript') || type.includes('xml')) {
    return { Icon: FileCode, text: 'text-sky-600', badge: 'text-sky-600 bg-sky-50 border-sky-100' };
  }
  if (type.includes('encrypted') || type.includes('pkcs')) {
    return { Icon: FileLock, text: 'text-yellow-700', badge: 'text-yellow-700 bg-yellow-50 border-yellow-100' };
  }

  return { Icon: FilePlus, text: 'text-muted-foreground', badge: 'text-muted-foreground bg-muted border-transparent' };
}

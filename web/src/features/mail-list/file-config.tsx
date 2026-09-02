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

import {
  FileText,
  FileImage,
  Video as FileVideo,
  FileArchive,
  FileSpreadsheet,
  FileCode,
  File as FileIcon,
  FileMusic as FileAudio,
} from 'lucide-react';

export const getFileConfig = (mimeType: string) => {
  const type = mimeType.toLowerCase();
  if (type.includes('pdf')) {
    return { icon: <FileText className="h-4 w-4" />, color: 'text-red-600 bg-red-50 border-red-100' };
  }
  if (type.includes('image/')) {
    return { icon: <FileImage className="h-4 w-4" />, color: 'text-blue-600 bg-blue-50 border-blue-100' };
  }
  if (type.includes('audio/')) {
    return { icon: <FileAudio className="h-4 w-4" />, color: 'text-purple-600 bg-purple-50 border-purple-100' };
  }

  if (type.includes('video/')) {
    return { icon: <FileVideo className="h-4 w-4" />, color: 'text-indigo-600 bg-indigo-50 border-indigo-100' };
  }
  if (type.includes('spreadsheet') || type.includes('excel') || type.includes('csv')) {
    return { icon: <FileSpreadsheet className="h-4 w-4" />, color: 'text-green-600 bg-green-50 border-green-100' };
  }
  if (type.includes('zip') || type.includes('compressed') || type.includes('archive')) {
    return { icon: <FileArchive className="h-4 w-4" />, color: 'text-orange-600 bg-orange-50 border-orange-100' };
  }
  if (type.includes('text/') || type.includes('json') || type.includes('javascript')) {
    return { icon: <FileCode className="h-4 w-4" />, color: 'text-slate-600 bg-slate-50 border-slate-100' };
  }

  return { icon: <FileIcon className="h-4 w-4" />, color: 'text-gray-600 bg-gray-50 border-gray-100' };
};

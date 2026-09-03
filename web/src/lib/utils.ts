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

import { enUS, zhCN, zhTW, arSA, de, es, fi, fr, it, ja, ko, nl, ptBR, ru, da, sv, nb } from 'date-fns/locale';
import { format, formatDistanceToNow } from 'date-fns';
import type { Locale } from 'date-fns';
import { type ClassValue, clsx } from 'clsx'
import { twMerge } from 'tailwind-merge'
import i18n from '@/i18n'

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}


export const formatBytes = (sizeInBytes: number): string => {
  if (sizeInBytes < 1024) {
    return `${sizeInBytes} B`;
  } else if (sizeInBytes < 1024 * 1024) {
    return `${(sizeInBytes / 1024).toFixed(2)} KB`;
  } else if (sizeInBytes < 1024 * 1024 * 1024) {
    return `${(sizeInBytes / (1024 * 1024)).toFixed(2)} MB`;
  } else {
    return `${(sizeInBytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  }
};



export function mapToRecordOfArrays(
  map: Map<number, Set<string>>
): Record<number, string[]> {
  return Object.fromEntries(
    Array.from(map.entries()).map(([key, value]) => [key, Array.from(value)])
  );
}

export function formatNumber(num: number): string {
  const userLocale = navigator.language;

  return new Intl.NumberFormat(userLocale, {
    maximumFractionDigits: 2,
  }).format(num);
}



export function validateTag(facetPath: string) {
  if (!facetPath || facetPath.length === 0) {
    return {
      valid: false,
      error: "Tag path cannot be empty"
    };
  }

  if (!facetPath.startsWith('/')) {
    return {
      valid: false,
      error: "Tag path must start with '/'"
    };
  }

  let escaped = false;
  for (let i = 1; i < facetPath.length; i++) {
    const char = facetPath[i];

    if (escaped) {
      escaped = false;
    } else if (char === '\\') {
      escaped = true;
    }
  }

  if (escaped) {
    return {
      valid: false,
      error: "Tag path has unmatched escape character at the end"
    };
  }

  return { valid: true };
}



// i18n.language -> date-fns locale
export const dateFnsLocaleMap: Record<string, Locale> = {
  en: enUS,
  'en-us': enUS,
  zh: zhCN,
  'zh-cn': zhCN,
  'zh-tw': zhTW,
  'zh_hk': zhTW,
  ar: arSA,
  'ar-sa': arSA,
  de: de,
  'de-de': de,
  es: es,
  'es-es': es,
  fi: fi,
  'fi-fi': fi,
  fr: fr,
  'fr-fr': fr,
  it: it,
  'it-it': it,
  jp: ja,
  ja: ja,
  'ja-jp': ja,
  ko: ko,
  'ko-kr': ko,
  nl: nl,
  'nl-nl': nl,
  pt: ptBR,
  'pt-br': ptBR,
  ru: ru,
  'ru-ru': ru,
  da: da,
  'da-dk': da,
  sv: sv,
  'sv-se': sv,
  no: nb,
  'no-no': nb,
};


function resolveLocale(locale?: Locale): Locale {
  return locale ?? dateFnsLocaleMap[i18n.language.toLowerCase()] ?? enUS;
}

export function formatDateTime(input: number | string | Date, locale?: Locale): string {
  const date = new Date(input);
  if (isNaN(date.getTime())) {
    return '';
  }
  return format(date, 'Pp', { locale: resolveLocale(locale) });
}

export function formatRelativeTime(input: number | string | Date, locale?: Locale): string {
  const date = new Date(input);
  if (isNaN(date.getTime())) {
    return '';
  }
  return formatDistanceToNow(date, { addSuffix: true, locale: resolveLocale(locale) });
}


export function showNumbers(current: number, total: number) {
  const max = 5
  const result = []

  if (total <= max) {
    for (let i = 1; i <= total; i++) {
      result.push(i)
    }
  } else {
    result.push(1)
    if (current <= 3) {
      for (let i = 2; i <= 4; i++) {
        result.push(i)
      }
      result.push('...', total)
    } else if (current >= total - 2) {
      result.push('...')
      for (let i = total - 3; i <= total; i++) {
        result.push(i)
      }
    } else {
      result.push('...')
      for (let i = current - 1; i <= current + 1; i++) {
        result.push(i)
      }
      result.push('...', total)
    }
  }
  return result
}


export function toSearchParams(obj: object): URLSearchParams {
  const params = new URLSearchParams();
  Object.entries(obj).forEach(([key, value]) => {
    if (value !== undefined && value !== null) {
      params.append(key, String(value));
    }
  });
  return params;
}
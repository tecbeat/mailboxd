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

import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useTranslation } from 'react-i18next'

// Size presets shared by every mail-list-style "more filters" popover. Each
// preset maps to a min/max byte range; `any` clears the range.
export const SIZES = {
  tiny: { min: undefined, max: 15 * 1024 },
  small: { min: undefined, max: 2 * 1024 * 1024 },
  medium: { min: 2 * 1024 * 1024, max: 10 * 1024 * 1024 },
  large: { min: 10 * 1024 * 1024, max: 20 * 1024 * 1024 },
  huge: { min: 20 * 1024 * 1024, max: undefined },
}

export type SizePreset = keyof typeof SIZES | 'any'

// Maps a concrete min/max range back to the preset key it came from, so the
// select can reflect the currently applied filter.
export const getPresetFromSize = (min?: number, max?: number): SizePreset => {
  if (min === SIZES.huge.min) return 'huge'
  if (min === SIZES.large.min && max === SIZES.large.max) return 'large'
  if (min === SIZES.medium.min && max === SIZES.medium.max) return 'medium'
  if (!min && max === SIZES.small.max) return 'small'
  if (!min && max === SIZES.tiny.max) return 'tiny'
  return 'any'
}

interface SizePresetSelectProps {
  label: string
  value: SizePreset
  onChange: (value: SizePreset) => void
}

// The size-preset dropdown. Feature-specific only in its section label; the
// options and their i18n keys are shared.
export function SizePresetSelect({ label, value, onChange }: SizePresetSelectProps) {
  const { t } = useTranslation()
  return (
    <div className="space-y-2">
      <Label className="text-xs text-muted-foreground">{label}</Label>
      <Select value={value} onValueChange={(v) => onChange(v as SizePreset)}>
        <SelectTrigger className="h-8 text-xs">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {Object.keys(SIZES)
            .concat('any')
            .map((key) => (
              <SelectItem key={key} className="text-xs" value={key}>
                {t(`search_more.size_presets.${key}`)}
              </SelectItem>
            ))}
        </SelectContent>
      </Select>
    </div>
  )
}

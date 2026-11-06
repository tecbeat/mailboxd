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

import {
  DndContext,
  KeyboardSensor,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
  type Announcements,
  type DragEndEvent,
  type UniqueIdentifier,
} from '@dnd-kit/core'
import {
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { GripVertical } from 'lucide-react'
import { useRef } from 'react'
import { useTranslation } from 'react-i18next'

import { Button } from '@/components/ui/button'
import { Switch } from '@/components/ui/switch'
import { cn } from '@/lib/utils'
import {
  TABLE_COLUMNS,
  defaultColumnSettings,
  moveColumn,
  type ColumnSetting,
  type ConfigurableTable,
} from '@/lib/table-columns'

interface ColumnSettingsProps {
  table: ConfigurableTable
  value: ColumnSetting[]
  onChange: (value: ColumnSetting[]) => void
}

/**
 * The columns of a table as a list: drag a row (or focus its handle and use
 * space and the arrow keys) to change the order, and switch columns on or
 * off. Columns that cannot be hidden keep a locked switch.
 */
export function ColumnSettings({ table, value, onChange }: ColumnSettingsProps) {
  const { t } = useTranslation()
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 4 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  )
  const columns = new Map<string, { labelKey: string; hideable: boolean }>(
    TABLE_COLUMNS[table].map((column) => [column.id, column]),
  )
  const label = (id: UniqueIdentifier) => {
    const column = columns.get(String(id))
    return column ? t(column.labelKey) : String(id)
  }
  const lastOver = useRef<UniqueIdentifier | null>(null)
  const position = (id: UniqueIdentifier | undefined) =>
    value.findIndex((setting) => setting.id === id) + 1

  const announcements: Announcements = {
    onDragStart: ({ active }) => {
      lastOver.current = active.id
      return t('settings.appearance.columns.dnd.start', { column: label(active.id), position: position(active.id) })
    },
    // dnd-kit also reports the start position as a drag over; announce only
    // real moves so the pickup message is not replaced right away.
    onDragOver: ({ active, over }) => {
      if (!over || over.id === lastOver.current) return undefined
      lastOver.current = over.id
      return t('settings.appearance.columns.dnd.over', { column: label(active.id), position: position(over.id) })
    },
    onDragEnd: ({ active, over }) =>
      over
        ? t('settings.appearance.columns.dnd.end', { column: label(active.id), position: position(over.id) })
        : t('settings.appearance.columns.dnd.cancel', { column: label(active.id) }),
    onDragCancel: ({ active }) => t('settings.appearance.columns.dnd.cancel', { column: label(active.id) }),
  }

  const onDragEnd = ({ active, over }: DragEndEvent) => {
    const next = moveColumn(value, String(active.id), over ? String(over.id) : undefined)
    if (next !== value) onChange(next)
  }

  const setVisible = (id: string, visible: boolean) =>
    onChange(value.map((setting) => (setting.id === id ? { ...setting, visible } : setting)))

  return (
    <div className='flex w-[400px] max-w-full flex-col gap-3'>
      <DndContext
        sensors={sensors}
        collisionDetection={closestCenter}
        onDragEnd={onDragEnd}
        accessibility={{
          announcements,
          screenReaderInstructions: { draggable: t('settings.appearance.columns.dnd.instructions') },
        }}
      >
        <SortableContext items={value.map((setting) => setting.id)} strategy={verticalListSortingStrategy}>
          <ul className='flex flex-col divide-y rounded-md border' aria-label={t('settings.appearance.field.account_columns')}>
            {value.map((setting) => (
              <ColumnRow
                key={setting.id}
                setting={setting}
                label={label(setting.id)}
                hideable={columns.get(setting.id)?.hideable ?? false}
                onVisibleChange={(visible) => setVisible(setting.id, visible)}
              />
            ))}
          </ul>
        </SortableContext>
      </DndContext>
      <div>
        <Button type='button' variant='outline' size='sm' onClick={() => onChange(defaultColumnSettings(table))}>
          {t('settings.appearance.columns.reset')}
        </Button>
      </div>
    </div>
  )
}

interface ColumnRowProps {
  setting: ColumnSetting
  label: string
  hideable: boolean
  onVisibleChange: (visible: boolean) => void
}

function ColumnRow({ setting, label, hideable, onVisibleChange }: ColumnRowProps) {
  const { t } = useTranslation()
  const { attributes, listeners, setNodeRef, setActivatorNodeRef, transform, transition, isDragging } =
    useSortable({ id: setting.id })

  return (
    <li
      ref={setNodeRef}
      data-column-id={setting.id}
      style={{ transform: CSS.Translate.toString(transform), transition }}
      className={cn(
        // `relative` keeps the hidden input of the switch inside the row, so
        // it cannot stretch the app shell and add a second scrollbar.
        'relative flex items-center gap-3 bg-background px-3 py-2 text-sm',
        isDragging && 'z-10 shadow-md',
      )}
    >
      <button
        type='button'
        ref={setActivatorNodeRef}
        {...attributes}
        {...listeners}
        aria-label={t('settings.appearance.columns.move', { column: label })}
        className='cursor-grab touch-none rounded-sm text-muted-foreground hover:text-foreground focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring active:cursor-grabbing'
      >
        <GripVertical className='h-4 w-4' />
      </button>
      <span className={cn('flex-1', !setting.visible && 'text-muted-foreground')}>{label}</span>
      {!hideable && (
        <span className='text-xs text-muted-foreground'>{t('settings.appearance.columns.always_visible')}</span>
      )}
      <Switch
        checked={hideable ? setting.visible : true}
        disabled={!hideable}
        onCheckedChange={onVisibleChange}
        aria-label={t('settings.appearance.columns.show', { column: label })}
      />
    </li>
  )
}

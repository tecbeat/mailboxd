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
import { z } from 'zod'
import { FormProvider, useForm } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import i18n from '@/i18n'
import { renderWithProviders } from '@/test/utils'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import { Form } from '@/components/ui/form'
import { RetentionField, type RetentionFormValues } from '../retention-field'
import {
  getRetentionDaysSchema,
  MAX_RETENTION_DAYS,
  parseRetentionDays,
} from '../schema'

const WARNING = /permanently deleted from the archive/

/** Render the field in a form; `latest` always holds the current form values. */
function renderField(defaults: RetentionFormValues = {}) {
  const latest: { values?: RetentionFormValues } = {}
  function Harness() {
    const form = useForm<RetentionFormValues>({
      mode: 'onChange',
      defaultValues: defaults,
      resolver: zodResolver(
        z.object({ retention_days: getRetentionDaysSchema(i18n.t) })
      ),
    })
    latest.values = form.watch()
    return (
      <FormProvider {...form}>
        <Form {...form}>
          <form>
            <RetentionField />
          </form>
        </Form>
      </FormProvider>
    )
  }
  renderWithProviders(<Harness />)
  return { latest, input: screen.getByLabelText('Retention period (days)') }
}

describe('parseRetentionDays', () => {
  it('turns an empty or blank field into no retention', () => {
    expect(parseRetentionDays('')).toBeUndefined()
    expect(parseRetentionDays('  ')).toBeUndefined()
  })

  it('reads the typed number of days', () => {
    expect(parseRetentionDays('30')).toBe(30)
    expect(parseRetentionDays('1.5')).toBe(1.5)
  })
})

describe('getRetentionDaysSchema', () => {
  const schema = getRetentionDaysSchema((key) => key)

  it.each([undefined, 1, 30, MAX_RETENTION_DAYS])('accepts %s', (days) => {
    expect(schema.safeParse(days).success).toBe(true)
  })

  it.each([0, -1, 1.5, MAX_RETENTION_DAYS + 1, Number.NaN])(
    'rejects %s with the range message',
    (days) => {
      const result = schema.safeParse(days)
      expect(result.success).toBe(false)
      expect(result.error?.issues[0].message).toBe(
        'validation.retentionDaysRange'
      )
    }
  )

  it('matches the server limit', () => {
    expect(MAX_RETENTION_DAYS).toBe(36500)
  })
})

describe('RetentionField', () => {
  it('starts empty, without a warning, when retention is off', () => {
    const { latest, input } = renderField()
    expect(input).toHaveValue(null)
    expect(input).toHaveAttribute('placeholder', 'Keep forever')
    expect(screen.queryByText(WARNING)).not.toBeInTheDocument()
    expect(latest.values?.retention_days).toBeUndefined()
  })

  it('shows the stored period and warns that older mail is deleted', () => {
    const { input } = renderField({ retention_days: 90 })
    expect(input).toHaveValue(90)
    expect(screen.getByText(WARNING)).toBeInTheDocument()
  })

  it('stores typed days as a number and warns', async () => {
    const { latest, input } = renderField()
    await userEvent.type(input, '30')
    expect(latest.values?.retention_days).toBe(30)
    expect(screen.getByText(WARNING)).toBeInTheDocument()
  })

  it('turns retention off again when the field is cleared', async () => {
    const { latest, input } = renderField({ retention_days: 30 })
    await userEvent.clear(input)
    expect(latest.values?.retention_days).toBeUndefined()
    expect(screen.queryByText(WARNING)).not.toBeInTheDocument()
  })

  it('explains the allowed range for an out-of-range value', async () => {
    const { input } = renderField()
    await userEvent.type(input, '0')
    await waitFor(() =>
      expect(
        screen.getByText(
          'Enter a whole number of days from 1 to 36500, or leave the field empty.'
        )
      ).toBeInTheDocument()
    )
  })
})

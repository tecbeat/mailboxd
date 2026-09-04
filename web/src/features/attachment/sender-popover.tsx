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

import * as React from "react"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"
import { Button } from "@/components/ui/button"
import { Mail } from "lucide-react"
import { useTranslation } from 'react-i18next'
import { filterSegment, FilterLabel, ActiveDot } from "@/features/mail-list/filter-bar"
import { useAttachmentContext } from "./context"
import { useAttachmentSenders } from "@/hooks/use-attachment-senders"
import { Group } from "@/api/system/api"
import { MetadataSelectorField } from "@/features/mail-list/attachment-metadata-selector"

export function SenderFilterPopover() {
    const { t } = useTranslation()
    const { filter, setFilter } = useAttachmentContext()
    const { senders, isLoading } = useAttachmentSenders("")

    const activeCount = filter.from ? 1 : 0
    const senderOptions: Group[] = React.useMemo(() => {
        return senders.map(email => ({
            key: email,
            count: 0
        }))
    }, [senders])

    const updateFilter = (email: string | undefined) => {
        setFilter(prev => ({
            ...prev,
            from: email
        }))
    }

    return (
        <Popover>
            <PopoverTrigger asChild>
                <Button
                    variant="ghost"
                    className={filterSegment(activeCount > 0)}
                    title={t('attachment.sender')}
                >
                    <Mail className="h-4 w-4" />
                    <FilterLabel>{t('attachment.sender')}</FilterLabel>
                    <ActiveDot active={activeCount > 0} />
                </Button>
            </PopoverTrigger>

            <PopoverContent
                align="start"
                className="w-fit min-w-[280px] max-w-[90vw] sm:max-w-[min(90vw,500px)] p-0 flex flex-col divide-y divide-border shadow-xl"
            >
                <div className="flex flex-col bg-muted/20">
                    <MetadataSelectorField
                        label={t('attachment.sender')}
                        value={filter.from}
                        options={senderOptions}
                        isLoading={isLoading}
                        onSelect={(val) => updateFilter(val)}
                        onReset={() => updateFilter(undefined)}
                    />
                </div>
            </PopoverContent>
        </Popover>
    )
}
import * as React from "react"
import { useTranslation } from "react-i18next"
import { useAttachmentContext } from "./context"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"
import { Button } from "@/components/ui/button"
import { MetadataSelectorField } from "@/features/mail-list/attachment-metadata-selector"
import { useAttachmentMetadata } from "@/hooks/use-attachment-metadata"
import { FilterLabel, ActiveDot } from "@/features/mail-list/filter-bar"
import { filterSegment } from "@/features/mail-list/filter-bar-styles"

interface MetaFilterProps {
    type: 'extension' | 'category' | 'content_type'
    icon: React.ReactNode
}

export function MetadataFilter({ type, icon }: MetaFilterProps) {
    const { t } = useTranslation()
    const { filter, setFilter } = useAttachmentContext()
    const [open, setOpen] = React.useState(false)
    const { data: meta, isLoading } = useAttachmentMetadata(open)

    const filterKey = `attachment_${type}` as const
    const currentValue = filter[filterKey] as string

    const optionsMap = {
        extension: meta?.extensions || [],
        category: meta?.categories || [],
        content_type: meta?.content_types || []
    }

    const handleSelect = (value: string | undefined) => {
        setFilter(prev => {
            const next = { ...prev }
            delete next.attachment_extension
            delete next.attachment_category
            delete next.attachment_content_type
            if (value) {
                next[filterKey] = value
            }

            return next
        })
        setOpen(false)
    }

    const handleReset = () => {
        setFilter(prev => {
            const next = { ...prev }
            delete next[filterKey]
            return next
        })
        setOpen(false)
    }

    return (
        <Popover open={open} onOpenChange={setOpen}>
            <PopoverTrigger asChild>
                <Button
                    variant="ghost"
                    className={filterSegment(!!currentValue)}
                    title={currentValue || t(`search_more.${type}`)}
                >
                    {icon}
                    <FilterLabel>
                        {currentValue || t(`search_more.${type}`)}
                    </FilterLabel>
                    <ActiveDot active={!!currentValue} />
                </Button>
            </PopoverTrigger>
            <PopoverContent align="start" className="w-64 p-2 shadow-xl">
                <MetadataSelectorField
                    label={t(`search_more.${type}`)}
                    value={currentValue || ''}
                    options={optionsMap[type]}
                    isLoading={isLoading}
                    onSelect={handleSelect}
                    onReset={handleReset}
                />
            </PopoverContent>
        </Popover>
    )
}
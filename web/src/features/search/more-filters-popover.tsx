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
import { Label } from "@/components/ui/label"
import { Input } from "@/components/ui/input"
import { useTranslation } from "react-i18next"
import { useSearchContext } from "./context"
import { Checkbox } from "@/components/ui/checkbox"
import { useAttachmentMetadata } from "@/hooks/use-attachment-metadata"
import { MetadataSelectorField } from "@/features/mail-list/attachment-metadata-selector"
import { MoreFiltersShell } from "@/features/mail-list/more-filters-shell"
import { SizePresetSelect } from "@/features/mail-list/size-presets"
import { SIZES, getPresetFromSize } from "@/features/mail-list/sizes"

export function MoreFiltersPopover() {
    const { t } = useTranslation();
    const { filter, setFilter } = useSearchContext();
    const [open, setOpen] = React.useState(false);

    const { data: meta, isLoading: metaLoading } = useAttachmentMetadata(open);

    const [localState, setLocalState] = React.useState({
        attachment_name: filter?.attachment_name || '',
        attachment_extension: filter?.attachment_extension || '',
        attachment_category: filter?.attachment_category || '',
        attachment_content_type: filter?.attachment_content_type || '',
        message_id: filter?.message_id || '',
        size_preset: getPresetFromSize(filter?.min_size, filter?.max_size),
        has_attachment: filter?.has_attachment || false
    });

    // Re-sync local draft from the active filter whenever the popover opens or the
    // filter changes while open (adjust state during render).
    const [prevDeps, setPrevDeps] = React.useState({ open, filter });
    if (prevDeps.open !== open || prevDeps.filter !== filter) {
        setPrevDeps({ open, filter });
        if (open) {
            setLocalState({
                attachment_name: filter?.attachment_name || '',
                attachment_extension: filter?.attachment_extension || '',
                attachment_category: filter?.attachment_category || '',
                attachment_content_type: filter?.attachment_content_type || '',
                message_id: filter?.message_id || '',
                size_preset: getPresetFromSize(filter?.min_size, filter?.max_size),
                has_attachment: filter?.has_attachment || false
            });
        }
    }

    const handleApply = () => {
        setFilter(prev => {
            const next = { ...prev };

            if (localState.attachment_name) next.attachment_name = localState.attachment_name;
            else delete next.attachment_name;

            if (localState.attachment_extension) next.attachment_extension = localState.attachment_extension;
            else delete next.attachment_extension;

            if (localState.attachment_category) next.attachment_category = localState.attachment_category;
            else delete next.attachment_category;

            if (localState.attachment_content_type) next.attachment_content_type = localState.attachment_content_type;
            else delete next.attachment_content_type;

            if (localState.message_id) next.message_id = localState.message_id;
            else delete next.message_id;

            if (localState.has_attachment) next.has_attachment = true;
            else delete next.has_attachment;

            const range = SIZES[localState.size_preset as keyof typeof SIZES] || { min: undefined, max: undefined };
            if (range.min) next.min_size = range.min; else delete next.min_size;
            if (range.max) next.max_size = range.max; else delete next.max_size;

            return next;
        });
        setOpen(false);
    };

    const handleReset = () => {
        setFilter(prev => {
            const next = { ...prev };
            delete next.attachment_name;
            delete next.min_size;
            delete next.max_size;
            delete next.message_id;
            delete next.has_attachment;
            delete next.attachment_extension;
            delete next.attachment_category;
            delete next.attachment_content_type;
            return next;
        });
        setOpen(false);
    };

    const activeCount = [
        filter?.attachment_name,
        filter?.min_size,
        filter?.max_size,
        filter?.message_id,
        filter?.has_attachment,
        filter?.attachment_extension,
        filter?.attachment_category,
        filter?.attachment_content_type
    ].filter(Boolean).length;

    return (
        <MoreFiltersShell
            open={open}
            onOpenChange={setOpen}
            activeCount={activeCount}
            triggerLabel={t('search_more.trigger_label')}
            title={t('search_more.title')}
            resetLabel={t('search_more.reset')}
            applyLabel={t('search_more.apply')}
            onReset={handleReset}
            onApply={handleApply}
        >
            <div className="flex items-center space-x-2 px-1">
                <Checkbox
                    id="has_attachment"
                    checked={localState.has_attachment}
                    onCheckedChange={(checked) => {
                        const isChecked = checked as boolean;
                        setLocalState(prev => ({
                            ...prev,
                            has_attachment: isChecked,
                            ...(isChecked ? {} : {
                                attachment_name: '',
                                attachment_extension: '',
                                attachment_category: '',
                                attachment_content_type: ''
                            })
                        }));
                    }}
                />
                <Label
                    htmlFor="has_attachment"
                    className="text-xs font-normal cursor-pointer select-none"
                >
                    {t('search_more.has_attachment')}
                </Label>
            </div>

            {localState.has_attachment && (
                <div className="space-y-3 p-2 bg-muted/30 rounded-lg border border-dashed border-border animate-in fade-in slide-in-from-top-1">
                    <MetadataSelectorField
                        label={t('search_more.extension')}
                        value={localState.attachment_extension}
                        options={meta?.extensions || []}
                        isLoading={metaLoading}
                        onSelect={(v) => setLocalState(p => ({ ...p, attachment_extension: v, attachment_category: '', attachment_content_type: '' }))}
                        onReset={() => setLocalState(p => ({ ...p, attachment_extension: '' }))}
                    />

                    <MetadataSelectorField
                        label={t('search_more.category')}
                        value={localState.attachment_category}
                        options={meta?.categories || []}
                        isLoading={metaLoading}
                        onSelect={(v) => setLocalState(p => ({ ...p, attachment_category: v, attachment_extension: '', attachment_content_type: '' }))}
                        onReset={() => setLocalState(p => ({ ...p, attachment_category: '' }))}
                    />

                    <MetadataSelectorField
                        label={t('search_more.content_type')}
                        value={localState.attachment_content_type}
                        options={meta?.content_types || []}
                        isLoading={metaLoading}
                        onSelect={(v) => setLocalState(p => ({ ...p, attachment_content_type: v, attachment_extension: '', attachment_category: '' }))}
                        onReset={() => setLocalState(p => ({ ...p, attachment_content_type: '' }))}
                    />

                    <div className="space-y-1 px-1">
                        <Label className="text-xs text-muted-foreground">{t('search_more.attachment_name_label')}</Label>
                        <Input
                            className="h-8 text-xs"
                            value={localState.attachment_name}
                            onChange={(e) => setLocalState(prev => ({ ...prev, attachment_name: e.target.value }))}
                            placeholder={t('search_more.attachment_name_placeholder')}
                        />
                    </div>
                </div>
            )}

            <SizePresetSelect
                label={t('search_more.message_size_label')}
                value={localState.size_preset}
                onChange={(v) => setLocalState(prev => ({ ...prev, size_preset: v }))}
            />

            <div className="space-y-2">
                <Label className="text-xs text-muted-foreground">{t('search_more.message_id_label')}</Label>
                <Input
                    className="h-8 text-xs"
                    value={localState.message_id}
                    onChange={(e) => setLocalState(prev => ({ ...prev, message_id: e.target.value }))}
                />
                <p className="text-[10px] text-muted-foreground opacity-70 leading-tight">
                    {t('search_more.message_id_description')}
                </p>
            </div>
        </MoreFiltersShell>
    );
}

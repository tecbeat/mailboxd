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
import { Info } from "lucide-react"
import { useTranslation } from "react-i18next"
import { useAttachmentContext } from "./context"
import { Checkbox } from "@/components/ui/checkbox"
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip"
import { MoreFiltersShell } from "@/features/mail-list/more-filters-shell"
import { SIZES, SizePresetSelect, getPresetFromSize } from "@/features/mail-list/size-presets"

export function MoreFiltersPopover() {
    const { t } = useTranslation();
    const { filter, setFilter } = useAttachmentContext();
    const [open, setOpen] = React.useState(false);

    const [localState, setLocalState] = React.useState({
        size_preset: getPresetFromSize(filter?.min_size, filter?.max_size),
        is_message: filter?.is_message || false
    });

    // Re-sync local draft from the active filter whenever the popover opens or the
    // filter changes while open (adjust state during render).
    const [prevDeps, setPrevDeps] = React.useState({ open, filter });
    if (prevDeps.open !== open || prevDeps.filter !== filter) {
        setPrevDeps({ open, filter });
        if (open) {
            setLocalState({
                size_preset: getPresetFromSize(filter?.min_size, filter?.max_size),
                is_message: filter?.is_message || false
            });
        }
    }

    const handleApply = () => {
        setFilter(prev => {
            const next = { ...prev };

            if (localState.is_message) next.is_message = true;
            else delete next.is_message;

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
            delete next.min_size;
            delete next.max_size;
            delete next.is_message;
            return next;
        });
        setOpen(false);
    };

    const activeCount = [
        filter?.min_size,
        filter?.max_size,
        filter?.is_message,
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
                    id="is_message"
                    checked={localState.is_message}
                    onCheckedChange={(checked) => {
                        const isChecked = checked as boolean;
                        setLocalState(prev => ({
                            ...prev,
                            is_message: isChecked
                        }));
                    }}
                />
                <Label
                    htmlFor="is_message"
                    className="text-xs font-normal cursor-pointer select-none"
                >
                    {t('search_more.is_message')}
                </Label>
                <TooltipProvider>
                    <Tooltip>
                        <TooltipTrigger asChild>
                            <Info className="w-3 h-3 ml-1.5 text-muted-foreground cursor-help" />
                        </TooltipTrigger>
                        <TooltipContent>
                            <p className="max-w-xs">{t('search_more.is_message_desc')}</p>
                        </TooltipContent>
                    </Tooltip>
                </TooltipProvider>
            </div>

            <SizePresetSelect
                label={t('attachment.size')}
                value={localState.size_preset}
                onChange={(v) => setLocalState(prev => ({ ...prev, size_preset: v }))}
            />
        </MoreFiltersShell>
    );
}

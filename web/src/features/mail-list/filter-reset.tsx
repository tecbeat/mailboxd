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


import { X } from "lucide-react"
import { Button } from "@/components/ui/button"
import { useMailListConfig } from "@/features/mail-list/config"
import { cn } from "@/lib/utils"
import { useTranslation } from "react-i18next";
import { FilterLabel, FilterCount } from "@/features/mail-list/filter-bar"
import { filterSegment } from "@/features/mail-list/filter-bar-styles"

export function FilterResetButton() {
    const { useListContext } = useMailListConfig();
    const { filter, setFilter } = useListContext();
    const { t } = useTranslation()
    const { q, ...restFilters } = filter;

    const activeFiltersCount = Object.keys(restFilters).filter(key => {
        const value = restFilters[key];
        if (Array.isArray(value)) return value.length > 0;
        return value !== undefined && value !== null && value !== '';
    }).length;

    if (activeFiltersCount === 0) return null;

    return (
        <Button
            variant="ghost"
            onClick={() => setFilter(q ? { q } : {})}
            className={cn(
                filterSegment(false),
                "text-muted-foreground hover:text-destructive"
            )}
            title={t('search_reset.tooltip')}
        >
            <X className="h-4 w-4" />
            <FilterLabel>{t('search_reset.label')}</FilterLabel>
            <FilterCount count={activeFiltersCount} />
        </Button>
    );
}
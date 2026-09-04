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

import React, { useState, useEffect, useRef } from "react"
import { Input } from "@/components/ui/input"
import { Button } from "@/components/ui/button"
import { Search, X, Clock, Trash2, LetterText } from "lucide-react"
import { cn } from "@/lib/utils"
import { useMailListConfig } from "@/features/mail-list/config"
import { useTranslation } from "react-i18next"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { FilterLabel } from "@/features/mail-list/filter-bar"

const MAX_HISTORY = 20

// One selectable search scope. `descKey`, when present, renders a secondary
// description line (used for the "all fields" option).
export interface TextSearchOption {
    value: string
    labelKey: string
    descKey?: string
}

// Feature-specific search configuration. Supplied by each feature's toolbar so
// the shared input can serve both search (subject/body) and attachment
// (attachment_name/from) scopes without knowing about either.
export interface TextSearchConfig {
    storageKey: string
    searchFields: string[]
    placeholderKey: string
    options: TextSearchOption[]
}

export function TextSearchInput({ config }: { config: TextSearchConfig }) {
    const { storageKey, searchFields, placeholderKey, options } = config
    const { t } = useTranslation()
    const { useListContext } = useMailListConfig()
    const { filter, setFilter } = useListContext()

    const [value, setValue] = useState("")
    const [field, setField] = useState<string>(options[0]?.value ?? "text")
    const [history, setHistory] = useState<string[]>(() => {
        try {
            const saved = localStorage.getItem(storageKey)
            return saved ? JSON.parse(saved) : []
        } catch {
            return []
        }
    })
    const [showHistory, setShowHistory] = useState(false)

    const inputRef = useRef<HTMLInputElement>(null)
    const containerRef = useRef<HTMLDivElement>(null)

    // Sync the active field/value from the current filter on mount and whenever the
    // filter or search fields change (adjust state during render).
    const [prevSync, setPrevSync] = useState<{ filter: typeof filter; searchFields: string[] } | null>(null)
    if (!prevSync || prevSync.filter !== filter || prevSync.searchFields !== searchFields) {
        setPrevSync({ filter, searchFields })
        const activeField = searchFields.find(key => !!filter[key]) || "text"
        const activeValue = filter[activeField] as string || ""
        setField(activeField)
        setValue(activeValue)
    }

    const applyFilter = (currentField: string, searchTerm: string) => {
        const trimmed = searchTerm.trim()

        setFilter((prev) => {
            const next = { ...prev }
            searchFields.forEach(f => {
                delete next[f]
            })
            if (trimmed) {
                next[currentField] = trimmed
            }
            return next
        })

        if (trimmed) {
            saveToHistory(trimmed)
        }
        setShowHistory(false)
        inputRef.current?.blur()
    }

    const saveToHistory = (term: string) => {
        setHistory((prev) => {
            const trimmed = term.trim()
            const newHistory = [trimmed, ...prev.filter((item) => item !== trimmed)].slice(0, MAX_HISTORY)
            localStorage.setItem(storageKey, JSON.stringify(newHistory))
            return newHistory
        })
    }

    const handleSearch = () => applyFilter(field, value)

    const handleClear = () => {
        setValue("")
        applyFilter(field, "")
    }

    const handleSelectHistory = (term: string) => {
        setValue(term)
        applyFilter(field, term)
    }

    const handleClearHistory = (e: React.MouseEvent) => {
        e.stopPropagation()
        setHistory([])
        localStorage.removeItem(storageKey)
    }

    useEffect(() => {
        const handleClickOutside = (e: MouseEvent) => {
            if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
                setShowHistory(false)
            }
        }
        document.addEventListener("mousedown", handleClickOutside)
        return () => document.removeEventListener("mousedown", handleClickOutside)
    }, [])

    // Live search: debounce the current term into the filter so results update
    // as the user types — no button and no extra click. Pressing Enter still
    // applies immediately and additionally records the term in the history.
    useEffect(() => {
        const trimmed = value.trim()
        const activeField = searchFields.find((key) => !!filter[key])
        const currentTerm = activeField ? (filter[activeField] as string) : ""
        // Already reflected in the filter (e.g. right after a sync): do nothing.
        if (trimmed === currentTerm && (!trimmed || activeField === field)) {
            return
        }
        const handle = setTimeout(() => {
            setFilter((prev) => {
                const next = { ...prev }
                searchFields.forEach((f) => {
                    delete next[f]
                })
                if (trimmed) {
                    next[field] = trimmed
                }
                return next
            })
        }, 300)
        return () => clearTimeout(handle)
    }, [value, field, filter, searchFields, setFilter])

    // The search unit grows to share one row while the bar is wide (`@xl/fbar`);
    // once the bar is narrow enough to wrap, `basis-full` claims the whole first
    // row so the search stays usable and the filters pack onto the next row
    // instead of leaving a large empty gap.
    return (
        <div ref={containerRef} className="relative flex flex-1 basis-full items-stretch @xl/fbar:basis-0">
            <Select
                value={field}
                onValueChange={(val) => {
                    setField(val)
                    if (value.trim()) applyFilter(val, value)
                }}
            >
                <SelectTrigger
                    className={cn(
                        "h-9 w-auto gap-1.5 rounded-none border-0 border-r border-border px-2.5",
                        "bg-transparent text-xs font-normal text-muted-foreground shadow-none focus:ring-0 focus:ring-offset-0"
                    )}
                >
                    <LetterText className="h-4 w-4 shrink-0" />
                    {/* `contents` wrapper keeps FilterLabel out of the trigger's
                        direct `[&>span]` line-clamp rule so its container-query
                        collapse (icon-only when the bar is narrow) still works. */}
                    <div className="contents">
                        <FilterLabel><SelectValue /></FilterLabel>
                    </div>
                </SelectTrigger>
                <SelectContent className="min-w-[220px]">
                    {options.map((opt) => (
                        <SelectItem
                            key={opt.value}
                            value={opt.value}
                            className="cursor-pointer text-xs"
                        >
                            {t(opt.labelKey)}
                        </SelectItem>
                    ))}
                </SelectContent>
            </Select>
            <div className="relative flex flex-1 min-w-[160px] items-center">
                <button
                    type="button"
                    onClick={handleSearch}
                    aria-label={t("search_input.button")}
                    className="absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground transition-colors hover:text-foreground"
                >
                    <Search className="h-4 w-4" />
                </button>
                <Input
                    ref={inputRef}
                    value={value}
                    onChange={(e) => setValue(e.target.value)}
                    onFocus={() => setShowHistory(true)}
                    onKeyDown={(e) => e.key === "Enter" && handleSearch()}
                    placeholder={t(placeholderKey)}
                    className="h-9 w-full rounded-none border-0 bg-transparent pl-8 pr-8 text-xs md:text-xs shadow-none focus-visible:ring-0"
                />
                {value && (
                    <Button
                        variant="ghost"
                        size="icon"
                        className="absolute right-1 top-1/2 h-7 w-7 -translate-y-1/2 text-muted-foreground hover:text-foreground"
                        onClick={handleClear}
                    >
                        <X className="h-4 w-4" />
                    </Button>
                )}
            </div>
            {showHistory && (
                <div className="absolute top-full left-0 w-full mt-1 bg-popover border rounded-md shadow-lg z-50 max-h-[300px] overflow-hidden flex flex-col">
                    <div className="py-2 px-3 text-[10px] uppercase tracking-wider text-muted-foreground font-semibold border-b flex items-center justify-between bg-muted/30">
                        <div className="flex items-center gap-1.5">
                            <Clock className="h-3 w-3" />
                            {t("search_input.recent_title")}
                        </div>
                        {history.length > 0 && (
                            <button
                                onClick={handleClearHistory}
                                className="text-destructive hover:underline flex items-center gap-1"
                            >
                                <Trash2 className="h-3 w-3" />
                                {t("search_input.clear_history")}
                            </button>
                        )}
                    </div>

                    <div className="overflow-auto py-1">
                        {history.length > 0 ? (
                            history.map((term, idx) => (
                                <button
                                    key={idx}
                                    className="w-full text-left px-3 py-2 text-sm hover:bg-accent transition-colors flex items-center gap-2 group"
                                    onClick={() => handleSelectHistory(term)}
                                >
                                    <Search className="h-3.5 w-3.5 text-muted-foreground group-hover:text-primary" />
                                    <span className="truncate flex-1 text-xs">{term}</span>
                                </button>
                            ))
                        ) : (
                            <div className="px-3 py-6 text-sm text-center text-muted-foreground">
                                {t("search_input.no_history")}
                            </div>
                        )}
                    </div>
                </div>
            )}
        </div>
    )
}
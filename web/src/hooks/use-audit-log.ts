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


import { PaginatedResponse } from '@/api';
import { AuditEntry, list_audit_log } from '@/api/audit/api';
import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { getRouteApi } from '@tanstack/react-router';
import React from 'react';

const routeApi = getRouteApi('/_authenticated/audit-log/')

export function useAuditLog() {
  const search = routeApi.useSearch()
  const navigate = routeApi.useNavigate()

  const page = search.page;
  const pageSize = search.pageSize;

  const updateParams = React.useCallback((newParams: Partial<typeof search>) => {
    navigate({
      search: (prev) => ({ ...prev, ...newParams }),
      replace: false,
    });
  }, [navigate]);

  const setPage = (p: number) => updateParams({ page: p });
  const setPageSize = (size: number) => updateParams({ pageSize: size, page: 1 });

  const { data, isLoading, isError, error, isFetching } = useQuery<
    PaginatedResponse<AuditEntry>
  >({
    queryKey: ['audit-log', page, pageSize],
    queryFn: () => list_audit_log(page, pageSize),
    // Keep the previous page mounted while the next page resolves so paging
    // doesn't blank the table to skeletons on every navigation.
    placeholderData: keepPreviousData,
    staleTime: 1000,
    retry: false,
  });

  return {
    entries: data?.items ?? [],
    total: data?.total_items ?? 0,
    totalPages: data?.total_pages ?? 1,
    pageSize: data?.page_size ?? pageSize,
    page,
    setPage,
    setPageSize,
    isLoading,
    isFetching,
    isError,
    error: error as Error | null,
  };
}

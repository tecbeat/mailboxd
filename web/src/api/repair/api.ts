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

import axiosInstance from "@/api/axiosInstance";

/**
 * `failed` arrives with HTTP 200: it is a verdict on the record, not a
 * transport error.
 */
export type BlobRepairStatus = 'complete' | 'repaired' | 'partial' | 'failed'

export interface BlobRepairReport {
    envelope_id: string
    account_id: number
    mailbox_id: number
    subject: string
    message_id: string
    status: BlobRepairStatus
    email_missing_before: boolean
    email_missing_after: boolean
    /** Attachment content hashes missing before the repair. */
    missing_before: string[]
    /** Attachment content hashes still missing after the repair. */
    missing_after: string[]
    error: string | null
}

/** One recorded repair attempt; `complete` results are never recorded. */
export interface BlobRepairRecord {
    id: string
    /** UTC millis when the repair finished. */
    created_at: number
    operator_id: number
    operator: string
    account_id: number
    mailbox_id: number
    subject: string
    message_id: string
    report: BlobRepairReport
}

export const REPAIR_REPORTS_KEY = ['repair-reports'] as const

/**
 * Re-fetches the message from the IMAP server and stores any missing content
 * that matches the archived hash. Can take a while; the 60s axios timeout
 * covers it.
 */
export const repairMessage = async (accountId: number, envelopeId: string) => {
    const response = await axiosInstance.post<BlobRepairReport>(
        `api/v1/repair-message/${accountId}/${envelopeId}`
    );
    return response.data;
};

/** The most recent repair records, newest first. */
export const listRepairReports = async (limit: number) => {
    const response = await axiosInstance.get<BlobRepairRecord[]>(
        'api/v1/repair-reports',
        { params: { limit } }
    );
    return response.data;
};

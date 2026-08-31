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

import { useAttachmentContext } from './context';
import { MailDisplayDrawer } from './mail-display-dialog';
import { EnvelopeDeleteDialog } from '@/features/mail-list/delete-dialog';
import { RestoreMessageDialog } from './restore-message-dialog';
import { NestedEmailDialog } from './nested-email-dialog';
import { MailBoxDeleteDialog } from '@/features/mail-list/delete-mailbox-dialog';

/**
 * All modal dialogs for the attachment view, keyed off the `open` value in
 * AttachmentContext. Kept as one component so the wiring is testable in
 * isolation and every dialog type has exactly one render site.
 */
export function AttachmentDialogs() {
  const { open, setOpen } = useAttachmentContext();

  return (
    <>
      <MailDisplayDrawer
        key="attachment-mail-display"
        open={open === 'display'}
        onOpenChange={() => setOpen('display')}
      />

      <EnvelopeDeleteDialog
        key="delete-attachment-envelope"
        open={open === 'delete'}
        onOpenChange={() => setOpen('delete')}
      />

      <RestoreMessageDialog
        key="attachment-restore-mail-dialog"
        open={open === 'restore'}
        onOpenChange={() => setOpen('restore')}
      />

      <NestedEmailDialog
        key="nested-eml-attachment-dialog"
        open={open === 'nested-eml'}
        onOpenChange={() => setOpen('nested-eml')}
      />

      <MailBoxDeleteDialog
        key="attachment-delete-mailbox"
        open={open === 'delete-mailbox'}
        onOpenChange={() => setOpen('delete-mailbox')}
      />
    </>
  );
}

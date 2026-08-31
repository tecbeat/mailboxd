//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
// Copyright (c) 2026 tecbeat
//
// This file is part of mailboxd, a fork of the Bichon email archiving
// project. Modifications by tecbeat, 2026.

import { useAttachmentContext } from './context';
import { MailDisplayDrawer } from './mail-display-dialog';
import { EnvelopeDeleteDialog } from './delete-dialog';
import { RestoreMessageDialog } from './restore-message-dialog';
import { NestedEmailDialog } from './nested-email-dialog';
import { MailBoxDeleteDialog } from './delete-mailbox-dialog';

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

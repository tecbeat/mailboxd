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

import { useState, type ReactNode } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { AxiosError } from "axios";
import { useTranslation } from "react-i18next";
import {
  Download,
  Upload,
  FileArchive,
  TriangleAlert as AlertTriangle,
  Clock,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Progress } from "@/components/ui/progress";
import { Spinner } from "@/components/ui/spinner";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { PageHeader } from "@/components/layout/page-header";
import { toast } from "@/hooks/use-toast";
import { cn, formatDateTime } from "@/lib/utils";
import {
  get_backup_config,
  create_backup,
  restore_backup,
  type BackupConfig,
} from "@/api/backup/api";

function StatusRow({
  label,
  value,
  mono,
}: {
  label: string;
  value: ReactNode;
  mono?: boolean;
}) {
  return (
    <div className="flex flex-col gap-1 border-b border-border/40 py-2.5 last:border-0 sm:flex-row sm:items-center sm:justify-between sm:gap-4">
      <div className="text-xs font-medium text-muted-foreground">{label}</div>
      <div className="flex min-w-0 sm:justify-end">
        {typeof value === "string" || typeof value === "number" ? (
          <span
            className={cn(
              "min-w-0 break-all text-sm font-medium sm:text-right",
              mono && "font-mono",
            )}
          >
            {value}
          </span>
        ) : (
          value
        )}
      </div>
    </div>
  );
}

function ScheduleStatus({ config }: { config: BackupConfig }) {
  const { t } = useTranslation();
  return (
    <div>
      <StatusRow
        label={t("backup.schedule.enabled", "Scheduled backups")}
        value={
          config.scheduled_enabled ? (
            <Badge variant="secondary">
              {t("backup.schedule.on", "Enabled")}
            </Badge>
          ) : (
            <Badge variant="outline" className="opacity-70">
              {t("backup.schedule.off", "Disabled")}
            </Badge>
          )
        }
      />
      <StatusRow
        label="MAILBOXD_BACKUP_DIR"
        value={config.backup_dir || "—"}
        mono
      />
      <StatusRow
        label="MAILBOXD_BACKUP_SCHEDULE"
        value={config.schedule || "—"}
        mono
      />
      <StatusRow
        label="MAILBOXD_BACKUP_RETENTION"
        value={
          config.retention === 0
            ? t("backup.schedule.keepAll", "Keep all")
            : String(config.retention)
        }
      />
      <StatusRow
        label={t("backup.schedule.count", "Scheduled archives on disk")}
        value={String(config.scheduled_count)}
      />
      <StatusRow
        label={t("backup.schedule.last", "Last scheduled backup")}
        value={
          config.last_backup_at
            ? formatDateTime(config.last_backup_at)
            : t("backup.schedule.never", "Never")
        }
      />
    </div>
  );
}

export default function BackupPage() {
  const { t } = useTranslation();
  const [file, setFile] = useState<File | null>(null);
  const [uploadPct, setUploadPct] = useState(0);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [restarting, setRestarting] = useState(false);

  const {
    data: config,
    isLoading,
    isError,
  } = useQuery({
    queryKey: ["backup-config"],
    queryFn: get_backup_config,
    retry: false,
  });

  const createMutation = useMutation({
    mutationFn: create_backup,
    onSuccess: () => {
      toast({
        title: t("backup.create.startedTitle", "Backup created"),
        description: t(
          "backup.create.startedDesc",
          "The backup has been created and your download should begin shortly.",
        ),
      });
    },
    onError: (err: AxiosError<{ message?: string }>) => {
      toast({
        title: t("common.failed", "Failed"),
        description: err.response?.data?.message || err.message,
        variant: "destructive",
      });
    },
  });

  const restoreMutation = useMutation({
    mutationFn: async () => {
      if (!file) return;
      setUploadPct(0);
      return restore_backup(file, (pct) => setUploadPct(pct));
    },
    onSuccess: () => {
      setRestarting(true);
      toast({
        title: t("backup.restore.stagedTitle", "Restore staged"),
        description: t(
          "backup.restore.stagedDesc",
          "The archive was accepted. The server is restarting to apply it.",
        ),
      });
    },
    onError: (err: AxiosError<{ message?: string }>) => {
      toast({
        title: t("common.failed", "Failed"),
        description: err.response?.data?.message || err.message,
        variant: "destructive",
      });
    },
  });

  const pickFile = () => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".zst,.tar.zst,application/zstd";
    input.onchange = () => {
      if (input.files && input.files[0]) setFile(input.files[0]);
    };
    input.click();
  };

  if (isError || (!isLoading && !config)) {
    return (
      <div className="p-8 text-center text-sm text-destructive">
        {t("backup.loadError", "Failed to load backup settings.")}
      </div>
    );
  }

  return (
    <div className="w-full">
      <PageHeader
        className="mb-4"
        title={t("backup.title", "Backup")}
        description={t(
          "backup.description",
          "Create and restore full server backups.",
        )}
      />

      {isLoading || !config ? (
        <div className="flex items-center justify-center py-24">
          <Spinner />
        </div>
      ) : (
        <div className="space-y-5">
          {/* Scheduled backups status */}
          <Card>
            <CardHeader className="flex flex-row items-center gap-3 border-b bg-muted/20 px-5 py-4">
              <div className="rounded-md bg-primary/10 p-1.5">
                <Clock className="h-3.5 w-3.5 text-primary" />
              </div>
              <div className="space-y-0.5">
                <CardTitle className="text-sm font-semibold">
                  {t("backup.schedule.title", "Scheduled backups")}
                </CardTitle>
                <CardDescription className="text-[11px]">
                  {t(
                    "backup.schedule.desc",
                    "Automatic backups are configured through environment variables.",
                  )}
                </CardDescription>
              </div>
            </CardHeader>
            <CardContent className="px-5 py-1">
              <ScheduleStatus config={config} />
            </CardContent>
          </Card>

          {/* Create backup */}
          <Card>
            <CardHeader className="pb-3">
              <CardTitle className="text-sm font-medium">
                {t("backup.create.title", "Create a backup")}
              </CardTitle>
              <CardDescription className="text-xs">
                {t(
                  "backup.create.desc",
                  "Download a full archive of all archived mail, attachments, search index and settings.",
                )}
              </CardDescription>
            </CardHeader>
            <CardContent>
              <Button
                onClick={() => createMutation.mutate()}
                disabled={createMutation.isPending}
                className="gap-2"
              >
                <Download
                  className={cn(
                    "h-4 w-4",
                    createMutation.isPending && "animate-pulse",
                  )}
                />
                {createMutation.isPending
                  ? t("backup.create.working", "Creating backup…")
                  : t("backup.create.button", "Create & download backup")}
              </Button>
            </CardContent>
          </Card>

          {/* Restore backup */}
          <Card>
            <CardHeader className="pb-3">
              <CardTitle className="text-sm font-medium">
                {t("backup.restore.title", "Restore from a backup")}
              </CardTitle>
              <CardDescription className="text-xs">
                {t(
                  "backup.restore.desc",
                  "Upload a backup archive. All current data is replaced and the server restarts to apply it.",
                )}
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex items-start gap-2 rounded-md border border-destructive/30 bg-destructive/5 p-3 text-xs text-destructive">
                <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" />
                <span>
                  {t(
                    "backup.restore.warning",
                    "Restoring replaces ALL current data and restarts the server. Encrypted archives can only be restored with the matching encryption password.",
                  )}
                </span>
              </div>

              <div className="flex flex-wrap items-center gap-3">
                <Button
                  variant="outline"
                  onClick={pickFile}
                  disabled={restoreMutation.isPending || restarting}
                  className="gap-2"
                >
                  <FileArchive className="h-4 w-4" />
                  {t("backup.restore.choose", "Choose archive…")}
                </Button>
                {file && (
                  <span className="truncate text-xs text-muted-foreground">
                    {file.name}
                  </span>
                )}
              </div>

              {restoreMutation.isPending && (
                <div className="space-y-1.5">
                  <div className="flex justify-between text-xs text-muted-foreground">
                    <span>{t("backup.restore.uploading", "Uploading…")}</span>
                    <span>{uploadPct}%</span>
                  </div>
                  <Progress value={uploadPct} className="h-2" />
                </div>
              )}

              {restarting && (
                <div className="rounded-md border border-border bg-muted/30 p-3 text-xs">
                  {t(
                    "backup.restore.restarting",
                    "The server is restarting to apply the restore. Please reload this page in a moment.",
                  )}
                </div>
              )}

              <div>
                <Button
                  variant="destructive"
                  onClick={() => setConfirmOpen(true)}
                  disabled={!file || restoreMutation.isPending || restarting}
                  className="gap-2"
                >
                  <Upload className="h-4 w-4" />
                  {t("backup.restore.button", "Restore this backup")}
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      )}

      <AlertDialog open={confirmOpen} onOpenChange={setConfirmOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>
              {t("backup.restore.confirmTitle", "Restore backup?")}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {t(
                "backup.restore.confirmDesc",
                "This permanently replaces all current data with the contents of the uploaded archive and restarts the server. This cannot be undone.",
              )}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>
              {t("common.cancel", "Cancel")}
            </AlertDialogCancel>
            <AlertDialogAction
              onClick={() => restoreMutation.mutate()}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              {t("backup.restore.confirmAction", "Restore and restart")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}

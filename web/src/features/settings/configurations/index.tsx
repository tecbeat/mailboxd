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
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { cn } from "@/lib/utils"
import { statusBadgeClass } from "@/lib/status-colors"
import { Spinner } from "@/components/ui/spinner"
import { PageHeader } from "@/components/layout/page-header"
import { ShieldCheck, Server, Database, Activity, Mail, Zap } from "lucide-react"
import { get_system_configurations } from "@/api/system/api"
import { useQuery } from "@tanstack/react-query"
import { useTranslation } from "react-i18next"

// const formatMB = (bytes?: number) => {
//   if (!bytes) return "—"
//   return `${(bytes / 1024 / 1024).toFixed(0)} MB`
// }

function BooleanBadge({ value }: { value: boolean }) {
  const { t } = useTranslation()
  return value ? (
    <Badge variant="secondary">{t("systemConfig.status.enabled")}</Badge>
  ) : (
    <Badge variant="outline" className="opacity-70">{t("systemConfig.status.disabled")}</Badge>
  )
}

function SettingRow({
  label,
  value,
  description,
  mono,
}: {
  label: string
  value: React.ReactNode
  description?: string
  mono?: boolean
}) {
  return (
    <div className="py-2.5 border-b border-border/40 last:border-0">
      <div className="flex flex-col gap-1 sm:flex-row sm:items-center sm:justify-between sm:gap-4">
        <div className="min-w-0 break-all text-xs font-medium text-muted-foreground font-mono">{label}</div>
        {/* Plain values get the same horizontal box (padding + transparent border) as a
            badge so that badge text and plain text share the exact same right edge. */}
        <div className="flex min-w-0 sm:justify-end">
          {React.isValidElement(value) ? (
            value
          ) : (
            <span
              className={cn(
                "min-w-0 break-all text-sm font-medium sm:border sm:border-transparent sm:px-2.5 sm:text-right",
                mono && "font-mono"
              )}
            >
              {value}
            </span>
          )}
        </div>
      </div>
      {description && (
        <div className="mt-1 text-[11px] text-muted-foreground">{description}</div>
      )}
    </div>
  )
}

function SettingsCard({
  icon: Icon,
  title,
  description,
  children,
}: {
  icon: React.ElementType
  title: string
  description?: string
  children: React.ReactNode
}) {
  return (
    <Card className="h-full">
      <CardHeader className="flex flex-row items-center gap-3 py-4 px-5 border-b bg-muted/20">
        <div className="p-1.5 rounded-md bg-primary/10">
          <Icon className="h-3.5 w-3.5 text-primary" />
        </div>
        <div className="space-y-0.5">
          <CardTitle className="text-sm font-semibold">{title}</CardTitle>
          {description && (
            <CardDescription className="text-[11px]">{description}</CardDescription>
          )}
        </div>
      </CardHeader>
      <CardContent className="px-5 py-1">{children}</CardContent>
    </Card>
  )
}

export default function ServerConfigurationsPage() {
  const { t } = useTranslation()
  const { data, isLoading, isError } = useQuery({
    queryKey: ["system-configurations"],
    queryFn: get_system_configurations,
  })

  if (isError || (!isLoading && !data)) {
    return (
      <div className="p-8 text-center text-sm text-destructive">
        {t("systemConfig.fields.loadError")}
      </div>
    )
  }

  return (
    <div className="w-full">
      <PageHeader
        className="mb-4"
        title={t("systemConfig.pageTitle")}
        description={t("systemConfig.pageDescription")}
      />

      {isLoading ? (
        <div className="flex justify-center items-center py-24">
          <Spinner />
        </div>
      ) : (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-5">
          <SettingsCard
            icon={Server}
            title={t("systemConfig.sections.network.title")}
            description={t("systemConfig.sections.network.desc")}
          >
            <SettingRow label="MAILBOXD_BIND_IP" value={data!.mailboxd_bind_ip ?? "0.0.0.0"} />
            <SettingRow label="MAILBOXD_HTTP_PORT" value={data!.mailboxd_http_port} />
            <SettingRow label="MAILBOXD_BASE_URL" value={data!.mailboxd_base_url} />
            <SettingRow label="MAILBOXD_PUBLIC_URL" value={data!.mailboxd_public_url} />
            <SettingRow
              label="MAILBOXD_ENABLE_REST_HTTPS"
              value={<BooleanBadge value={data!.mailboxd_enable_rest_https} />}
            />
          </SettingsCard>

          <SettingsCard
            icon={Mail}
            title={t("systemConfig.sections.smtp.title")}
            description={t("systemConfig.sections.smtp.desc")}
          >
            <SettingRow label="MAILBOXD_ENABLE_SMTP" value={<BooleanBadge value={data!.mailboxd_enable_smtp} />} />
            <SettingRow label="MAILBOXD_SMTP_PORT" value={data!.mailboxd_smtp_port} />
            <SettingRow label="MAILBOXD_SMTP_ENCRYPTION" value={data!.mailboxd_smtp_encryption} />
            <SettingRow label="MAILBOXD_SMTP_AUTH_REQUIRED" value={<BooleanBadge value={data!.mailboxd_smtp_auth_required} />} />
          </SettingsCard>

          <SettingsCard
            icon={Zap}
            title={t("systemConfig.sections.performance.title")}
            description={t("systemConfig.sections.performance.desc")}
          >
            <SettingRow label="MAILBOXD_SYNC_CONCURRENCY" value={data!.mailboxd_sync_concurrency ?? t("systemConfig.status.auto")} />
            <SettingRow
              label="MAILBOXD_HTTP_COMPRESSION_ENABLED"
              value={<BooleanBadge value={data!.mailboxd_http_compression_enabled} />}
            />
          </SettingsCard>

          <SettingsCard
            icon={Database}
            title={t("systemConfig.sections.storage.title")}
            description={t("systemConfig.sections.storage.desc")}
          >
            <SettingRow label="MAILBOXD_ROOT_DIR" value={data!.mailboxd_root_dir} mono />
            <SettingRow label="MAILBOXD_DATA_DIR" value={data!.mailboxd_data_dir || "—"} mono />
            <SettingRow label="MAILBOXD_INDEX_DIR" value={data!.mailboxd_index_dir || "—"} mono />
          </SettingsCard>

          <SettingsCard
            icon={ShieldCheck}
            title={t("systemConfig.sections.security.title")}
            description={t("systemConfig.sections.security.desc")}
          >
            <SettingRow
              label="MAILBOXD_ENCRYPT_PASSWORD_SET"
              value={
                data!.mailboxd_encrypt_password_set ? (
                  <Badge variant="outline" className={statusBadgeClass.success}>
                    {t("systemConfig.status.configured")}
                  </Badge>
                ) : (
                  <Badge variant="outline" className={statusBadgeClass.error}>{t("systemConfig.status.missing")}</Badge>
                )
              }
            />
            <SettingRow
              label="MAILBOXD_WEBUI_TOKEN_EXPIRATION_HOURS"
              value={`${data!.mailboxd_webui_token_expiration_hours}h`}
            />
          </SettingsCard>

          <SettingsCard
            icon={Activity}
            title={t("systemConfig.sections.logging.title")}
            description={t("systemConfig.sections.logging.desc")}
          >
            <SettingRow label="MAILBOXD_LOG_LEVEL" value={<Badge variant="outline" className="uppercase">{data!.mailboxd_log_level}</Badge>} />
            <SettingRow label="MAILBOXD_ANSI_LOGS" value={<BooleanBadge value={data!.mailboxd_ansi_logs} />} />
            <SettingRow label="MAILBOXD_JSON_LOGS" value={<BooleanBadge value={data!.mailboxd_json_logs} />} />
            <SettingRow label="MAILBOXD_LOG_TO_FILE" value={<BooleanBadge value={data!.mailboxd_log_to_file} />} />
            <SettingRow label="MAILBOXD_MAX_SERVER_LOG_FILES" value={data!.mailboxd_max_server_log_files} />
          </SettingsCard>
        </div>
      )}
    </div>
  )
}
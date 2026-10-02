//
// Copyright (c) 2026 tecbeat
//
// This file is part of mailboxd, a fork of the Bichon email archiving project.
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

import { useFormContext } from "react-hook-form";
import { useTranslation } from "react-i18next";
import {
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
  FormControl,
  FormDescription,
} from "@/components/ui/form";
import { Input } from "@/components/ui/input";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { PasswordInput } from "@/components/password-input";
import { AccountFormValues } from "./schema";
import useProxyList from "@/hooks/use-proxy";

interface TabServerJmapProps {
  isEdit?: boolean;
}

/**
 * JMAP server/auth fields. Parallels `tab-server.tsx` (IMAP) but for the JMAP
 * source: an optional session URL (resolved from the email via autodiscovery
 * when left blank) and a Basic/Bearer/OAuth2 auth method with conditionally
 * rendered credential inputs.
 */
export function TabServerJmap({ isEdit }: TabServerJmapProps) {
  const { t } = useTranslation();
  const { control, watch } = useFormContext<AccountFormValues>();
  const authType = watch('jmap.auth.auth_type');
  const { proxyOptions } = useProxyList();

  return (
    <div className="space-y-6">
      <FormField
        control={control}
        name="jmap.session_url"
        render={({ field }) => (
          <FormItem>
            <FormLabel>{t('accounts.jmapSessionUrl')}</FormLabel>
            <FormControl>
              <Input
                {...field}
                value={field.value ?? ''}
                placeholder={t('accounts.jmapSessionUrlPlaceholder')}
              />
            </FormControl>
            <FormDescription>{t('accounts.jmapSessionUrlDescription')}</FormDescription>
            <FormMessage />
          </FormItem>
        )}
      />

      <FormField
        control={control}
        name="jmap.auth.auth_type"
        render={({ field }) => (
          <FormItem>
            <FormLabel>{t('accounts.jmapAuthMethod')}<span className="text-destructive align-super text-xs">*</span></FormLabel>
            <Select onValueChange={field.onChange} value={field.value}>
              <FormControl>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
              </FormControl>
              <SelectContent>
                <SelectItem value="Basic">{t('accounts.jmapAuthBasic')}</SelectItem>
                <SelectItem value="Bearer">{t('accounts.jmapAuthBearer')}</SelectItem>
                <SelectItem value="OAuth2">OAuth2</SelectItem>
              </SelectContent>
            </Select>
            <FormMessage />
          </FormItem>
        )}
      />

      {authType === 'Basic' && (
        <FormField
          control={control}
          name="jmap.auth.username"
          render={({ field }) => (
            <FormItem>
              <FormLabel>{t('accounts.jmapUsername')}<span className="text-destructive align-super text-xs">*</span></FormLabel>
              <FormControl>
                <Input {...field} value={field.value ?? ''} placeholder={t('accounts.jmapUsernamePlaceholder')} />
              </FormControl>
              <FormMessage />
            </FormItem>
          )}
        />
      )}

      {(authType === 'Basic' || authType === 'Bearer') && (
        <FormField
          control={control}
          name="jmap.auth.secret"
          render={({ field }) => (
            <FormItem>
              <FormLabel>
                {authType === 'Basic' ? t('accounts.jmapPassword') : t('accounts.jmapToken')}
                {!isEdit && <span className="text-destructive align-super text-xs">*</span>}
              </FormLabel>
              <FormControl>
                <PasswordInput
                  placeholder={isEdit ? t('accounts.leaveEmptyToKeepSecret') : t('accounts.enterSecret')}
                  {...field}
                  value={field.value ?? ''}
                />
              </FormControl>
              {isEdit && <FormDescription>{t('accounts.leaveEmptyToKeepSecret')}</FormDescription>}
              <FormMessage />
            </FormItem>
          )}
        />
      )}

      <FormField
        control={control}
        name="jmap.use_proxy"
        render={({ field }) => (
          <FormItem>
            <FormLabel>{t('accounts.useProxyOptional')}</FormLabel>
            <Select
              onValueChange={(v) => field.onChange(v === 'none' ? undefined : Number(v))}
              defaultValue={field.value?.toString()}
            >
              <FormControl>
                <SelectTrigger>
                  <SelectValue placeholder={t('accounts.selectProxy')} />
                </SelectTrigger>
              </FormControl>
              <SelectContent>
                <SelectItem key="none" value="none">{t('accounts.useNoProxy')}</SelectItem>
                {proxyOptions.map((opt) => (
                  <SelectItem key={opt.value} value={opt.value}>
                    <span className="max-w-[280px] truncate block" title={opt.label}>
                      {opt.label}
                    </span>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <FormMessage />
          </FormItem>
        )}
      />

      <FormField
        control={control}
        name="use_dangerous"
        render={({ field }) => (
          <FormItem className="flex flex-row items-start space-x-3 space-y-0 rounded-md border p-4">
            <FormControl>
              <Checkbox checked={field.value} onCheckedChange={field.onChange} />
            </FormControl>
            <div className="space-y-1 leading-none">
              <FormLabel>{t('accounts.useDangerous')}</FormLabel>
            </div>
          </FormItem>
        )}
      />
    </div>
  );
}

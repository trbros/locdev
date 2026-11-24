// The Mailbox: captured messages and each site's wiring. The daemon parses
// messages and decides the wiring; nothing here sends mail anywhere.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { backend, request } from "@/lib/daemon";

import { errorMessage } from "./use-daemon";
import { t, tn } from "@/lib/i18n";

export interface MailFilter {
  site: string | null;
  query: string;
  unread: boolean;
  attachments: boolean;
}

export function useMailList(filter: MailFilter) {
  return useQuery({
    queryKey: ["mail", "list", filter],
    queryFn: async () =>
      (
        await request(
          {
            type: "mail_list",
            site: filter.site,
            query: filter.query || null,
            unread: filter.unread,
            attachments: filter.attachments,
            limit: 500,
          },
          "mail_list",
        )
      ).messages,
    // New mail shows up without a reload.
    refetchInterval: 3000,
    retry: false,
  });
}

export function useMailbox() {
  return useQuery({
    queryKey: ["mail", "status"],
    queryFn: async () => {
      const { type: _, ...status } = await request({ type: "mailbox_status" }, "mailbox_status");
      return status;
    },
    refetchInterval: 5000,
    retry: false,
  });
}

export function useMailDetail(id: string | null) {
  const client = useQueryClient();
  return useQuery({
    queryKey: ["mail", "detail", id],
    queryFn: async () => {
      const { type: _, ...detail } = await request({ type: "mail_get", id: id! }, "mail_detail");
      // Opening marks it read.
      void client.invalidateQueries({ queryKey: ["mail", "list"] });
      return detail;
    },
    enabled: id !== null,
    retry: false,
  });
}

export function useMailSetup(site: string) {
  return useQuery({
    queryKey: ["mail", "setup", site],
    queryFn: async () => {
      const { type: _, ...setup } = await request({ type: "mail_setup", name: site }, "mail_setup");
      return setup;
    },
    retry: false,
  });
}

export function useMailActions() {
  const client = useQueryClient();
  const refresh = () => client.invalidateQueries({ queryKey: ["mail"] });
  return {
    remove: useMutation({
      mutationFn: (ids: string[]) => request({ type: "mail_delete", ids }, "mail_deleted"),
      onSettled: refresh,
    }),
    clear: useMutation({
      mutationFn: (site: string | null) => request({ type: "mail_clear", site }, "mail_deleted"),
      onSuccess: (r) => toast.success(tn("Deleted {n} message", "Deleted {n} messages", r.count)),
      onSettled: refresh,
    }),
    enable: useMutation({
      mutationFn: (v: { name: string; enabled: boolean; override_real: boolean }) =>
        request({ type: "mail_enable", ...v }, "mail_setup"),
      onSuccess: (s) =>
        toast.success(
          s.enabled
            ? t("{site}'s mail goes to the Mailbox", { site: s.site })
            : t("{site}'s mail is back to its own settings", { site: s.site }),
          {
            description: t("Restart the site so its processes pick it up."),
          },
        ),
      onError: (e) => toast.error(t("Could not change the Mailbox"), { description: errorMessage(e) }),
      onSettled: () => {
        void refresh();
        void client.invalidateQueries({ queryKey: ["projects"] });
      },
    }),
    test: useMutation({
      mutationFn: (name: string | null) => request({ type: "mail_send_test", name }, "ok"),
      onSuccess: () => toast.success(t("Test message sent to the Mailbox")),
      onError: (e) => toast.error(t("Could not send"), { description: errorMessage(e) }),
      onSettled: refresh,
    }),
    save: async (id: string, attachment: number | null, fileName: string) => {
      try {
        const path = await backend.saveMailFile(id, attachment, fileName);
        if (path) toast.success(t("Saved"), { description: path });
      } catch (e) {
        toast.error(t("Could not save"), { description: errorMessage(e) });
      }
    },
  };
}

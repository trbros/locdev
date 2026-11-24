// Email Studio: the catalog, a template's segments and previews. The daemon
// builds the file text from segments (code kept as written) and saves only
// what was reviewed.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { request } from "@/lib/daemon";
import type { JsonValue, PreviewMode, Segment } from "@/lib/protocol";

export function useEmailCatalog(site: string) {
  return useQuery({
    queryKey: ["email", site, "catalog"],
    queryFn: async () => {
      const { type: _, ...catalog } = await request({ type: "email_catalog", name: site }, "email_catalog");
      return catalog;
    },
    retry: false,
  });
}

export async function openEmail(site: string, path: string) {
  const { type: _, ...doc } = await request({ type: "email_open", name: site, path }, "email_doc");
  return doc;
}

export async function emailStat(site: string, path: string) {
  return (await request({ type: "email_stat", name: site, path }, "email_stat")).hash;
}

export async function previewEmail(
  site: string,
  path: string,
  segments: Segment[] | null,
  theme: JsonValue | null,
  data: JsonValue,
  mode: PreviewMode,
) {
  const { type: _, ...preview } = await request(
    { type: "email_preview", name: site, path, segments, theme: theme ?? undefined, data, mode },
    "email_preview",
  );
  return preview;
}

export async function buildEmail(site: string, path: string, segments: Segment[], theme: JsonValue) {
  return request({ type: "email_build", name: site, path, segments, theme }, "email_built");
}

export function useEmailActions(site: string) {
  const client = useQueryClient();
  const refresh = () => client.invalidateQueries({ queryKey: ["email", site] });
  return {
    save: useMutation({
      mutationFn: (v: { path: string; content: string; baseHash: string }) =>
        request(
          { type: "email_save", name: site, path: v.path, content: v.content, base_hash: v.baseHash },
          "email_saved",
        ),
      onSettled: refresh,
    }),
    create: useMutation({
      mutationFn: (v: { component: boolean; item: string; markdown: boolean }) =>
        request({ type: "email_new", name: site, ...v }, "email_created"),
      onSettled: refresh,
    }),
    sample: (path: string, data: JsonValue) => request({ type: "email_sample", name: site, path, data }, "ok"),
    sendTest: useMutation({
      mutationFn: (v: { path: string; segments: Segment[]; theme: JsonValue; data: JsonValue; mode: PreviewMode }) =>
        request({ type: "email_send_test", name: site, ...v }, "ok"),
      onSettled: () => client.invalidateQueries({ queryKey: ["mail"] }),
    }),
  };
}

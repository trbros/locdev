// Code editors. The daemon finds them and starts them; this asks and shows.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { request } from "@/lib/daemon";
import type { EditorsView } from "@/lib/protocol";

import { errorMessage } from "./use-daemon";
import { t } from "@/lib/i18n";

export function useEditors() {
  return useQuery({
    queryKey: ["editors"],
    queryFn: async (): Promise<EditorsView> => {
      const { type: _, ...view } = await request({ type: "editor_list" }, "editors");
      return view;
    },
    staleTime: 5 * 60_000,
    retry: false,
  });
}

export function useEditorsRefresh() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: async (): Promise<EditorsView> => {
      const { type: _, ...view } = await request({ type: "editor_list", refresh: true }, "editors");
      return view;
    },
    onSuccess: (view) => client.setQueryData(["editors"], view),
  });
}

/** Opens a site's folder, or `path` inside it, in the chosen editor. */
export function useOpenInEditor() {
  return useMutation({
    mutationFn: (input: { site: string; path?: string; line?: number; editor?: string }) =>
      request({ type: "editor_open", ...input }, "ok"),
    onError: (error) => toast.error(t("Could not open the editor"), { description: errorMessage(error) }),
  });
}

/** The name of the editor that opens by default, for button labels. */
export function useDefaultEditorName() {
  const editors = useEditors();
  const id = editors.data?.default;
  return editors.data?.editors.find((e) => e.id === id)?.name ?? null;
}

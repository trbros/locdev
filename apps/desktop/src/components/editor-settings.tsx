import { useQueryClient } from "@tanstack/react-query";
import { Code2, RefreshCw } from "lucide-react";

import { Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { useSettings } from "@/hooks/use-daemon";
import { useEditors, useEditorsRefresh } from "@/hooks/use-editors";
import { cn } from "@/lib/utils";
import { t } from "@/lib/i18n";
import { tx } from "@/lib/i18n-jsx";

/** Settings → Integrations: which installed editor sites and files open in. */
export function EditorSection() {
  const editors = useEditors();
  const refresh = useEditorsRefresh();
  const { query, update } = useSettings();
  const view = refresh.data ?? editors.data;
  const settings = query.data;
  const client = useQueryClient();
  // The default editor comes from the daemon, which reads the setting.
  const choose = (id: string | null) =>
    settings &&
    update.mutate(
      { ...settings, editor: { ...settings.editor, preferred: id } },
      {
        onSuccess: () => {
          refresh.reset();
          client.invalidateQueries({ queryKey: ["editors"] });
        },
      },
    );

  return (
    <Section title={t("code editor")}>
      <p className="text-[13px] text-muted-foreground">
        {tx(
          "Sites and files open in this editor: from a site's menu, the Config and Email Studio tabs, or {cmd}. LocDev finds editors by their command-line launcher.",
          {
            cmd: <span className="font-mono text-foreground">locdev editor open</span>,
          },
        )}
      </p>
      {!view ? (
        <Skeleton className="mt-3 h-20" />
      ) : view.editors.length === 0 ? (
        <p className="mt-3 text-sm">
          {t(
            "No editor found. Install VS Code, Cursor, Zed, Sublime Text or a JetBrains IDE, or put its command on PATH (VS Code: “Shell Command: Install 'code' command in PATH”), then look again.",
          )}
        </p>
      ) : (
        <div role="radiogroup" aria-label={t("Code editor")} className="mt-3 grid grid-cols-2 gap-2">
          {view.editors.map((editor) => {
            const active = view.default === editor.id;
            return (
              <button
                type="button"
                role="radio"
                aria-checked={active}
                key={editor.id}
                onClick={() => choose(editor.id)}
                className={cn(
                  "flex items-start gap-2.5 rounded-lg border px-3 py-2 text-left transition-colors hover:bg-foreground/3",
                  active && "border-ember/60 bg-ember/6",
                )}
              >
                <Code2 className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
                <span className="min-w-0">
                  <span className="block text-sm">{editor.name}</span>
                  <span className="block truncate font-mono text-[11px] text-muted-foreground">{editor.program}</span>
                </span>
              </button>
            );
          })}
        </div>
      )}
      {view?.preferred && !view.editors.some((e) => e.id === view.preferred) && (
        <p className="mt-2 text-[13px] text-muted-foreground">
          {t("The chosen editor ({id}) isn't installed; the first one found opens instead.", { id: view.preferred })}
        </p>
      )}
      <div className="mt-3 flex gap-2">
        <Button variant="outline" size="sm" disabled={refresh.isPending} onClick={() => refresh.mutate()}>
          <RefreshCw /> {t("Look again")}
        </Button>
        {view?.preferred && (
          <Button variant="ghost" size="sm" onClick={() => choose(null)}>
            {t("Use the first one found")}
          </Button>
        )}
      </div>
    </Section>
  );
}

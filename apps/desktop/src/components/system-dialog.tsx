import { ShieldCheck } from "lucide-react";

import { CodeView } from "@/components/code-editor";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Skeleton } from "@/components/ui/skeleton";
import { type SystemChange, useSystemApply, useSystemPlan } from "@/hooks/use-daemon";
import type { Script } from "@/lib/protocol";
import { t } from "@/lib/i18n";

const TITLES: Record<string, string> = {
  "dns:install": "Route *.test to LocDev",
  "dns:uninstall": t("Remove LocDev's DNS changes"),
  "tls:install": t("Trust the LocDev certificate authority"),
  "tls:uninstall": t("Stop trusting the LocDev certificate authority"),
};

/**
 * Shows exactly what a privileged change will run, then hands it to the OS
 * authorization prompt. Nothing runs until the user presses Authorize.
 */
export function SystemDialog({ change, onClose }: { change: SystemChange | null; onClose: () => void }) {
  const plan = useSystemPlan(change);
  const apply = useSystemApply();

  const authorize = () => {
    if (change) apply.mutate(change, { onSuccess: onClose });
  };

  return (
    <Dialog open={change !== null} onOpenChange={(open) => !open && !apply.isPending && onClose()}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>{change ? TITLES[`${change.target}:${change.action}`] : ""}</DialogTitle>
          <DialogDescription>
            {plan.data?.summary ?? t("Preparing the change…")}{" "}
            {t("Your system will ask for an administrator password.")}
          </DialogDescription>
        </DialogHeader>

        {plan.isPending ? (
          <div className="space-y-2 rounded-lg border bg-well p-3">
            <Skeleton className="h-3 w-3/4" />
            <Skeleton className="h-3 w-1/2" />
            <Skeleton className="h-3 w-2/3" />
          </div>
        ) : plan.isError ? (
          <p className="rounded-lg border border-destructive/30 bg-destructive/[0.07] p-3 text-sm text-destructive">
            {(plan.error as Error).message}
          </p>
        ) : (
          plan.data && (
            <div className="flex min-w-0 flex-col gap-2">
              <ScriptBlock label={t("runs as administrator")} script={plan.data.script} />
              {plan.data.user_script && <ScriptBlock label={t("then runs as you")} script={plan.data.user_script} />}
            </div>
          )
        )}

        <DialogFooter>
          <Button variant="outline" onClick={onClose} disabled={apply.isPending}>
            {t("Cancel")}
          </Button>
          <Button
            onClick={authorize}
            disabled={!plan.data || apply.isPending}
            className="bg-ember text-ember-foreground hover:bg-ember/85"
          >
            <ShieldCheck data-icon="inline-start" />
            {apply.isPending ? t("Waiting for authorization…") : t("Authorize")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function ScriptBlock({ label, script }: { label: string; script: Script }) {
  return (
    <div className="min-w-0 overflow-hidden rounded-lg border bg-well">
      <div className="border-b px-3 py-1.5 font-mono text-[10.5px] text-muted-foreground">
        {label} · {script.shell === "power_shell" ? t("PowerShell") : "sh"}
      </div>
      <CodeView
        value={script.source}
        language={script.shell === "power_shell" ? "powershell" : "shell"}
        lineNumbers={false}
        ariaLabel={`${label} script`}
        className="max-h-48 rounded-none border-0"
      />
    </div>
  );
}

import { useState } from "react";
import { Led } from "@/components/status";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { SwitchField } from "@/components/ui/switch-field";
import { useProjectAction, useProjects } from "@/hooks/use-daemon";
import { tildify } from "@/lib/format";
import { useUi } from "@/lib/ui-state";
import { t } from "@/lib/i18n";
import { tx } from "@/lib/i18n-jsx";

/** Confirms removing the site named in `removing`, from any menu. */
export function RemoveSiteDialog() {
  const { removing, setRemoving } = useUi();
  const projects = useProjects();
  const action = useProjectAction();
  const site = projects.data?.find((p) => p.name === removing);
  const [deleteFiles, setDeleteFiles] = useState(false);

  const close = () => {
    setRemoving(null);
    setDeleteFiles(false);
  };
  const folder = <span className="font-mono">{site ? tildify(site.path) : ""}</span>;

  return (
    <Dialog open={!!removing} onOpenChange={(open) => !open && close()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2 font-mono">
            <Led tone="fail" /> {t("remove")} {site?.domain ?? removing}?
          </DialogTitle>
          <DialogDescription>
            {deleteFiles
              ? tx("LocDev stops the site, forgets its settings and moves the folder {folder} to the trash.", {
                  folder,
                })
              : tx(
                  "LocDev stops the site and forgets its settings. The folder {folder} and its files are not touched.",
                  {
                    folder,
                  },
                )}
            {site?.parked && (
              <>
                {" "}
                {t("It came from the parked folder {path}, where it will be ignored so it doesn't come back.", {
                  path: tildify(site.parked),
                })}
              </>
            )}
          </DialogDescription>
        </DialogHeader>
        {site && (
          <SwitchField
            className="flex items-start gap-3"
            checked={deleteFiles}
            onCheckedChange={setDeleteFiles}
            switchClassName="mt-0.5 data-checked:bg-destructive"
          >
            <span className="block text-sm">{t("Also move the folder to the trash")}</span>
            <span className="block text-[12.5px] text-muted-foreground">
              {t("Everything in it goes, uncommitted work included; it can be restored from the system's trash.")}
            </span>
          </SwitchField>
        )}
        <DialogFooter>
          <Button variant="outline" onClick={close}>
            {t("Cancel")}
          </Button>
          <Button
            variant="destructive"
            onClick={() => {
              if (removing) action.mutate({ action: "remove_project", name: removing, deleteFiles });
              close();
            }}
          >
            {deleteFiles ? t("Remove and delete the folder") : t("Remove")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

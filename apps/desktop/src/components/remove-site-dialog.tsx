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
import { useProjectAction, useProjects } from "@/hooks/use-daemon";
import { tildify } from "@/lib/format";
import { useUi } from "@/lib/ui-state";
import { t } from "@/lib/i18n";

/** Confirms removing the site named in `removing`, from any menu. */
export function RemoveSiteDialog() {
  const { removing, setRemoving } = useUi();
  const projects = useProjects();
  const action = useProjectAction();
  const site = projects.data?.find((p) => p.name === removing);

  const close = () => setRemoving(null);

  return (
    <Dialog open={!!removing} onOpenChange={(open) => !open && close()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2 font-mono">
            <Led tone="fail" /> {t("remove")} {site?.domain ?? removing}?
          </DialogTitle>
          <DialogDescription>
            {t("LocDev stops the site and forgets its settings. The folder")}{" "}
            <span className="font-mono">{site ? tildify(site.path) : ""}</span> {t("and its files are not touched.")}
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
        <DialogFooter>
          <Button variant="outline" onClick={close}>
            {t("Cancel")}
          </Button>
          <Button
            variant="destructive"
            onClick={() => {
              if (removing) action.mutate({ action: "remove_project", name: removing });
              close();
            }}
          >
            {t("Remove")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

import { Fact, Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { useAppInfo } from "@/hooks/use-daemon";
import { revealFolder } from "@/lib/system";
import { FolderOpen } from "lucide-react";
import { t } from "@/lib/i18n";

export function FoldersSection() {
  const info = useAppInfo();
  if (!info.data) return null;
  return (
    <Section title={t("files")}>
      <Fact label={t("Configuration")} mono>
        {info.data.config_dir}
      </Fact>
      <Fact label={t("Logs")} mono>
        {info.data.log_dir}
      </Fact>
      <div className="mt-3 flex gap-2">
        <Button variant="outline" size="sm" onClick={() => revealFolder(info.data.config_dir)}>
          <FolderOpen data-icon="inline-start" /> {t("Config folder")}
        </Button>
        <Button variant="outline" size="sm" onClick={() => revealFolder(info.data.log_dir)}>
          <FolderOpen data-icon="inline-start" /> {t("Logs folder")}
        </Button>
      </div>
    </Section>
  );
}

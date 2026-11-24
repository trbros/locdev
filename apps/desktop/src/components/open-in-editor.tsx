import { Code2 } from "lucide-react";

import { Button } from "@/components/ui/button";
import { useDefaultEditorName, useOpenInEditor } from "@/hooks/use-editors";
import { t } from "@/lib/i18n";

/** "Open in Zed" for a file inside a site (hidden when no editor is installed). */
export function OpenInEditorButton({ site, path, line }: { site: string; path: string; line?: number }) {
  const editor = useDefaultEditorName();
  const open = useOpenInEditor();
  if (!editor) return null;
  return (
    <Button
      size="xs"
      variant="ghost"
      disabled={open.isPending}
      onClick={() => open.mutate({ site, path, line })}
      title={t("Open in {editor}", { editor })}
    >
      <Code2 data-icon="inline-start" /> {editor}
    </Button>
  );
}

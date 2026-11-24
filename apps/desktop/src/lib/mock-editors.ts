import type { EditorRequest, EditorsView, Response } from "./protocol";

/** Two editors "installed" in the mock; the preferred one comes from settings. */
const editors = [
  { id: "vscode", name: "Visual Studio Code", program: "/usr/bin/code" },
  { id: "zed", name: "Zed", program: "/home/ana/.local/bin/zed" },
];

export function handleEditors(req: EditorRequest, preferred: () => string | null | undefined): Response {
  if (req.type === "editor_open") {
    if (req.path?.includes("..")) {
      return { type: "error", code: "INVALID_INPUT", message: `${req.path} isn't a file inside the site` } as Response;
    }
    return { type: "ok" } as Response;
  }
  const choice = preferred();
  const view: EditorsView = {
    editors,
    preferred: choice ?? null,
    default: editors.some((e) => e.id === choice) ? choice : editors[0].id,
  };
  return { type: "editors", ...view } as Response;
}

import { t } from "@/lib/i18n";

/**
 * Fixed texts the daemon sends that the app shows through t(): the steps of a
 * new project's plan (crates/locdev-scaffold/src/plan.rs). Never called; the
 * literal t() calls make the i18n check require their translations. The
 * frameworks' and tools' texts come from src/generated/ (see i18n-check.mjs).
 */
export function planStepLabels(): string[] {
  return [
    t("Add Actix Web"),
    t("Add Axum"),
    t("Add Tokio"),
    t("Add the chosen packages"),
    t("Add the web app pack"),
    t("Build the front-end"),
    t("Create the Cargo package"),
    t("Create the Django project"),
    t("Create the project"),
    t("Create the virtual environment"),
    t("Create the workspace"),
    t("Download WordPress"),
    t("Initialize a Git repository"),
    t("Install Django"),
    t("Install the front-end dependencies"),
    t("Run the migrations"),
  ];
}

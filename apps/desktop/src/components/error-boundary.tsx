import { Component, type ErrorInfo, type ReactNode } from "react";

import { Button } from "@/components/ui/button";
import { t } from "@/lib/i18n";

/** Keeps a crash in one screen from blanking the whole window. */
export class ErrorBoundary extends Component<{ children: ReactNode }, { error: Error | null }> {
  state = { error: null as Error | null };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error(error, info.componentStack);
  }

  render() {
    if (!this.state.error) return this.props.children;

    return (
      <div className="flex h-full flex-col items-center justify-center gap-3 p-8 text-center">
        <h1 className="text-lg font-semibold">{t("Something went wrong")}</h1>
        <pre
          className="max-w-xl overflow-auto rounded-lg border bg-card p-3 text-left font-mono text-xs text-destructive"
          data-selectable
        >
          {this.state.error.message}
        </pre>
        <Button variant="outline" onClick={() => this.setState({ error: null })}>
          {t("Try again")}
        </Button>
      </div>
    );
  }
}

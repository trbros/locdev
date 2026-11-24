import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ThemeProvider } from "next-themes";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import App from "@/App";
import { ErrorBoundary } from "@/components/error-boundary";
import { Toaster } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";
import { PrefsProvider } from "@/lib/prefs";
import { ShortcutsProvider } from "@/lib/shortcuts";
import { UiProvider } from "@/lib/ui-state";

import "./index.css";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: { refetchOnWindowFocus: true, structuralSharing: true },
  },
});

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ThemeProvider attribute="class" defaultTheme="dark" enableSystem disableTransitionOnChange>
      <QueryClientProvider client={queryClient}>
        <TooltipProvider>
          <PrefsProvider>
            <ShortcutsProvider>
              <UiProvider>
                <ErrorBoundary>
                  <App />
                </ErrorBoundary>
                <Toaster position="bottom-right" />
              </UiProvider>
            </ShortcutsProvider>
          </PrefsProvider>
        </TooltipProvider>
      </QueryClientProvider>
    </ThemeProvider>
  </StrictMode>,
);

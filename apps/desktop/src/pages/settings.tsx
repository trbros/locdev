import type { ReactNode } from "react";

import { IntegrationsSection, SecuritySection } from "@/components/account-settings";
import { DeployPlatformsSection } from "@/components/deploy-remote";
import { DebugSettingsSection } from "@/components/debug-settings";
import { EditorSection } from "@/components/editor-settings";
import { ShortcutsSettings } from "@/components/shortcuts-settings";
import { EnvironmentSettings } from "@/components/env";
import { Breadcrumbs, SideNav, useCrumbs } from "@/components/page";
import { ParkedFolders } from "@/components/parked-folders";
import { DefinitionsSettings } from "@/components/stack";
import { TlsSection } from "@/components/tls-section";
import { TrioooLogo } from "@/components/triooo-logo";
import { UpdatesSection } from "@/components/updates";
import { ScrollArea } from "@/components/ui/scroll-area";
import { SETTINGS, subById, viewById } from "@/lib/nav";
import { useUi } from "@/lib/ui-state";
import {
  AboutSection,
  AppearanceSection,
  AutomationsSection,
  CliSection,
  DaemonSection,
  DnsSection,
  DomainsSection,
  FoldersSection,
  OverviewSection,
  ProxySection,
  SiteHttpsSection,
} from "@/sections";
import { t } from "@/lib/i18n";

const SECTIONS: Record<string, () => ReactNode> = {
  overview: () => <OverviewSection />,
  daemon: () => (
    <>
      <DaemonSection />
      <FoldersSection />
    </>
  ),
  proxy: () => (
    <>
      <ProxySection />
      <DomainsSection />
    </>
  ),
  dns: () => <DnsSection />,
  parked: () => <ParkedFolders />,
  frameworks: () => <DefinitionsSettings />,
  environment: () => <EnvironmentSettings />,
  security: () => <SecuritySection />,
  integrations: () => (
    <>
      <IntegrationsSection />
      <EditorSection />
      <DeployPlatformsSection />
    </>
  ),
  https: () => (
    <>
      <TlsSection />
      <SiteHttpsSection />
    </>
  ),
  automations: () => (
    <>
      <AutomationsSection />
      <DebugSettingsSection />
    </>
  ),
  updates: () => <UpdatesSection />,
  appearance: () => <AppearanceSection />,
  shortcuts: () => <ShortcutsSettings />,
  about: () => (
    <>
      <AboutSection />
      <CliSection />
      <div className="flex flex-col items-center gap-2 text-muted-foreground mt-20">
        <span className="text-xs">{t("Developed and maintained by")}</span>
        <a href="https://triooo.com.br" target="_blank" rel="noopener" aria-label="triooo">
          <TrioooLogo className="w-16" />
        </a>
        <span className="text-xs">{t("and collaborators")}</span>
      </div>
    </>
  ),
};

export function SettingsView() {
  const view = viewById("settings");
  const { sub: current, navigate } = useUi();
  const sub = subById(view, current)!;
  const crumbs = useCrumbs(view, sub);

  return (
    <ScrollArea className="h-full">
      <div className="mx-auto flex max-w-5xl gap-10 px-10 pt-6 pb-12">
        <div className="sticky top-6 flex flex-col gap-6 self-start">
          <div>
            <h1 className="text-[22px] leading-none font-semibold tracking-[-0.02em]">{view.label}</h1>
          </div>
          <SideNav items={SETTINGS} value={sub.id} onChange={(id) => navigate("settings", id)} label={t("Settings")} />
        </div>
        <div key={sub.id} className="flex min-w-0 flex-1 animate-in flex-col gap-6 fade-in-0 duration-150">
          <header className="flex flex-col gap-3">
            <Breadcrumbs crumbs={crumbs} />
            <div>
              <h2 className="text-[22px] leading-none font-semibold tracking-[-0.02em]">{sub.label}</h2>
              <p className="mt-2 text-sm text-muted-foreground">{sub.description}</p>
            </div>
          </header>
          {SECTIONS[sub.id]?.()}
        </div>
      </div>
    </ScrollArea>
  );
}

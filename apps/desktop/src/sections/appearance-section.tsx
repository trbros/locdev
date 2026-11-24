import { useTheme } from "next-themes";
import { InterfaceThemeSelector } from "@/components/interface-theme-selector";
import { LanguageSelector } from "@/components/language-selector";
import { MotionPreferenceSelector } from "@/components/motion-preference-selector";
import { Section } from "@/components/page";
import { SidebarLayoutSelector } from "@/components/sidebar-layout-selector";
import { usePrefs } from "@/lib/prefs";
import { t } from "@/lib/i18n";
import { tx } from "@/lib/i18n-jsx";

export function AppearanceSection() {
  const { theme, setTheme } = useTheme();
  const prefs = usePrefs();

  return (
    <>
      <Section title={t("app language")}>
        <LanguageSelector />
      </Section>
      <Section title={t("theme")}>
        <InterfaceThemeSelector value={theme} onChange={setTheme} />
        {/* <Choices items={THEMES} value={theme} onChange={setTheme} /> */}
      </Section>
      <Section title={t("sidebar")}>
        <SidebarLayoutSelector value={prefs.sidebar} onChange={(value) => prefs.set("sidebar", value)} />
        <p className="mt-2 text-[13px] text-muted-foreground">
          {tx("Toggle it anytime with {ctrl_b}. Narrow windows always show icons only.", {
            ctrl_b: <span className="font-mono text-foreground">Ctrl B</span>,
          })}
        </p>
      </Section>
      <Section title={t("motion")}>
        <MotionPreferenceSelector value={prefs.motion} onChange={(value) => prefs.set("motion", value)} />
        <p className="mt-2 text-[13px] text-muted-foreground">
          {t("Reduced keeps state changes but drops entrance animations and the splash animation.")}
        </p>
      </Section>
    </>
  );
}

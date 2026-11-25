import { Laptop2, Monitor, Terminal } from "lucide-react";
import { getTranslations } from "next-intl/server";
import { Apple, Linux, Windows } from "@/components/icons";

export async function PlatformSection() {
	const t = await getTranslations("Platform");
	const l = await getTranslations("Labels");
	return (
		<section className="section-pad border-t border-border">
			<div className="page-container">
				<div className="section-topline">
					<p className="section-eyebrow">{t("eyebrow")}</p>
					<span className="font-mono text-[11px] text-muted-foreground">
						{l("crossPlatform")}
					</span>
				</div>
				<div className="section-intro">
					<h2 className="section-heading">{t("title")}</h2>
					<p className="section-description">{t("body")}</p>
				</div>
				<div className="platform-grid mt-12">
					{(
						[
							{
								name: t("linux"),
								logo: Linux,
								tag: "GNU / UNIX",
								icon: Terminal,
							},
							{ name: t("mac"), logo: Apple, tag: "DARWIN", icon: Laptop2 },
							{
								name: t("windows"),
								logo: Windows,
								tag: "WIN32",
								icon: Monitor,
							},
						] as const
					).map((p, i) => (
						<div className="platform-card" key={p.tag}>
							<span className="platform-index">
								{l("systemIndex", { index: `0${i + 1}` })}
							</span>
							<div className="platform-glyph" aria-hidden="true">
								<p.logo className="platform-logo" focusable="false" />
							</div>
							<div className="flex items-end justify-between gap-3">
								<div>
									<h3>{p.name}</h3>
									<p>{p.tag}</p>
								</div>
								<p.icon size={20} className="text-ember" />
							</div>
						</div>
					))}
				</div>
				<p className="mt-5 font-mono text-[11px] text-muted-foreground">
					* {t("note")}
				</p>
			</div>
		</section>
	);
}

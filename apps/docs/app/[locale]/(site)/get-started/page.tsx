import { Download, GitBranch, Terminal } from "lucide-react";
import type { Metadata } from "next";
import { getTranslations } from "next-intl/server";
import { InstallGuide } from "@/components/onboarding/install-guide";
import { getReleaseFeed } from "@/lib/github-releases";
export async function generateMetadata(): Promise<Metadata> {
	const t = await getTranslations("Onboarding");
	return { title: t("title"), description: t("body") };
}
export default async function GetStartedPage() {
	const t = await getTranslations("Onboarding");
	const feed = await getReleaseFeed();
	const n = await getTranslations("InstallationGuide");
	return (
		<div className="page-container tool-page installation-page">
			<header className="installation-hero">
				<p className="section-eyebrow">{t("eyebrow")}</p>
				<h1>
					{n("heroTitle")}
					<span>{n("heroAccent")}</span>
				</h1>
				<p className="route-description">{n("heroBody")}</p>
				<div className="installation-hero-paths">
					<span>
						<Download size={16} />
						{n("app")}
					</span>
					<span>
						<Terminal size={16} />
						{n("terminal")}
					</span>
					<span>
						<GitBranch size={16} />
						Git
					</span>
				</div>
			</header>
			<InstallGuide feed={feed} />
		</div>
	);
}

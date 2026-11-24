import type { Metadata } from "next";
import { getTranslations } from "next-intl/server";
import { ProjectGenerator } from "@/components/onboarding/project-generator";
export async function generateMetadata(): Promise<Metadata> {
	const t = await getTranslations("Generator");
	return { title: t("title"), description: t("body") };
}
export default async function ConfigurePage({
	searchParams,
}: {
	searchParams: Promise<{ stack?: string }>;
}) {
	const t = await getTranslations("Generator");
	const { stack } = await searchParams;
	const n = await getTranslations("PageImprovements");
	return (
		<div className="page-container tool-page configure-page">
			<header className="route-hero tool-hero">
				<p className="section-eyebrow">{t("eyebrow")}</p>
				<h1 className="section-heading">{t("title")}</h1>
				<p className="route-description">{t("body")}</p>
			</header>
			<div className="configure-intro">
				<h2>{n("configureTitle")}</h2>
				<p>{n("configureBody")}</p>
			</div>
			<ProjectGenerator initialStack={stack} />
		</div>
	);
}

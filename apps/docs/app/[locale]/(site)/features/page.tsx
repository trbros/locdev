import type { Metadata } from "next";
import { getTranslations } from "next-intl/server";
import { FeatureArchitecture } from "@/components/features/feature-architecture";
import { FeatureControl } from "@/components/features/feature-control";
import { FeatureHero } from "@/components/features/feature-hero";
import { FeatureSectionNav } from "@/components/features/feature-section-nav";
import { FeatureWorkflows } from "@/components/features/feature-workflows";

export async function generateMetadata(): Promise<Metadata> {
	const t = await getTranslations("FeaturePage");
	return { title: t("title"), description: t("body") };
}
export default function FeaturesPage() {
	return (
		<div className="features-page">
			<FeatureHero />
			<FeatureSectionNav />
			<FeatureArchitecture />
			<FeatureWorkflows />
			<FeatureControl />
		</div>
	);
}

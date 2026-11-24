import { ArrowRight, ArrowUpRight, Braces } from "lucide-react";
import { getTranslations } from "next-intl/server";
import { PageReveal } from "@/components/page-reveal";
import { ThemedShot } from "@/components/themed-shot";
import { Link } from "@/i18n/navigation";

export async function FeatureHero() {
	const t = await getTranslations("FeaturePage");
	return (
		<section className="route-hero page-container">
			<PageReveal className="route-hero-grid">
				<div className="route-hero-copy">
					<p className="section-eyebrow">{t("eyebrow")}</p>
					<h1>{t("title")}</h1>
					<p className="route-description">{t("body")}</p>
					<div className="route-actions">
						<Link href="/download" className="btn-ember">
							{t("primary")}
							<ArrowUpRight size={16} />
						</Link>
						<a href="#workflows" className="btn-outline">
							{t("workflowsLabel")}
							<ArrowRight size={16} />
						</a>
					</div>
				</div>
				<div className="feature-workbench">
					<div className="feature-workbench-title">
						<Braces size={16} />
						<span>{t("runtimesPreview")}</span>
						<span className="led led-green" />
					</div>
					<ThemedShot
						name="runtimes"
						alt={t("runtimesAlt")}
						width={1920}
						height={1200}
						sizes="(max-width:768px) 100vw,50vw"
						priority
					/>
					<div className="feature-workbench-footer">
						<code>locdev edit shop --node 22 --php 8.3</code>
					</div>
				</div>
			</PageReveal>
		</section>
	);
}

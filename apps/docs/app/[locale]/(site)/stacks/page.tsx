import { ArrowDown, ArrowUpRight } from "lucide-react";
import type { Metadata } from "next";
import { getTranslations } from "next-intl/server";
import { Suspense } from "react";
import { PageLoading } from "@/components/page-loading";
import { StackCatalog } from "@/components/stacks/stack-catalog";
import { frameworkRequestUrl, stacks } from "@/lib/stacks";

export async function generateMetadata(): Promise<Metadata> {
	const t = await getTranslations("Stacks");
	return { title: t("pageTitle"), description: t("pageBody") };
}
export default async function StacksPage() {
	const t = await getTranslations("Stacks");
	return (
		<div className="page-container stacks-page">
			<section className="route-hero stack-catalog-hero">
				<p className="section-eyebrow">{t("eyebrow")}</p>
				<h1 className="section-heading">{t("pageTitle")}</h1>
				<p className="route-description">{t("pageBody")}</p>
				<div className="route-actions">
					<a href="#stack-directory-title" className="btn-ember">
						{t("viewAll")}
						<ArrowDown size={16} />
					</a>
					<a
						href={frameworkRequestUrl()}
						className="btn-outline"
						target="_blank"
						rel="noopener noreferrer"
					>
						{t("request")}
						<ArrowUpRight size={16} />
					</a>
				</div>
				<dl className="catalog-summary">
					{(["detected", "script", "custom"] as const).map((support) => (
						<div key={support}>
							<dt>{t(`support.${support}`)}</dt>
							<dd>
								{stacks.filter((stack) => stack.support === support).length}
							</dd>
						</div>
					))}
				</dl>
			</section>
			<Suspense fallback={<PageLoading cards />}>
				<StackCatalog />
			</Suspense>
		</div>
	);
}

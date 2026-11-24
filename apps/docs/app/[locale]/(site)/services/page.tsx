import { ArrowDown, ArrowUpRight } from "lucide-react";
import type { Metadata } from "next";
import { getTranslations } from "next-intl/server";
import { ServiceCatalog } from "@/components/services/service-catalog";
import { serviceRequestUrl, services } from "@/lib/stack-services";
export async function generateMetadata(): Promise<Metadata> {
	const t = await getTranslations("Services");
	return { title: t("pageTitle"), description: t("body") };
}
export default async function ServicesPage() {
	const t = await getTranslations("Services");
	return (
		<div className="page-container services-page">
			<section className="route-hero stack-catalog-hero">
				<p className="section-eyebrow">{t("eyebrow")}</p>
				<h1 className="section-heading">{t("title")}</h1>
				<p className="route-description">{t("body")}</p>
				<p className="service-planned">{t("planned")}</p>
				<div className="route-actions">
					<a href="#service-directory-title" className="btn-ember">
						{t("viewAll")}
						<ArrowDown size={16} />
					</a>
					<a
						className="btn-outline"
						href={serviceRequestUrl("LocDev")}
						target="_blank"
						rel="noopener noreferrer"
					>
						{t("request")}
						<ArrowUpRight size={16} />
					</a>
				</div>
				<dl className="catalog-summary">
					{(["database", "background", "realtime", "tooling"] as const).map(
						(category) => (
							<div key={category}>
								<dt>{t(`categories.${category}`)}</dt>
								<dd>
									{
										services.filter((service) => service.category === category)
											.length
									}
								</dd>
							</div>
						),
					)}
				</dl>
			</section>
			<ServiceCatalog />
		</div>
	);
}

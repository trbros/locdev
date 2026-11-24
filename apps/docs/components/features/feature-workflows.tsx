import { ArrowUpRight, Check } from "lucide-react";
import { getTranslations } from "next-intl/server";
import { PageReveal } from "@/components/page-reveal";
import { ThemedShot } from "@/components/themed-shot";
import { Link } from "@/i18n/navigation";
import { route } from "@/lib/routes.generated";

export async function FeatureWorkflows() {
	const t = await getTranslations("FeatureDetails");
	const c = await getTranslations("Common");
	const workflows = [
		{
			key: "sites",
			image: "sites",
			href: route("docs.$slug", { slug: "sites" }),
			command: "locdev add . --start",
		},
		{
			key: "runtimes",
			image: "runtimes",
			href: route("docs.$slug", { slug: "runtimes" }),
			command: "locdev edit shop --node 22 --php 8.3",
		},
		{
			key: "doctor",
			image: "doctor",
			href: route("docs.$slug", { slug: "doctor" }),
			command: "locdev doctor",
		},
	] as const;
	return (
		<section
			id="workflows"
			className="feature-workflows border-t border-border"
		>
			<div className="page-container">
				<div className="route-section-heading feature-workflows-heading">
					<div>
						<p className="section-eyebrow">{t("eyebrow")}</p>
						<h2>{t("title")}</h2>
					</div>
				</div>
				{workflows.map((flow, index) => (
					<PageReveal
						key={flow.key}
						className={`feature-workflow ${index % 2 ? "feature-workflow-reverse" : ""}`}
					>
						<div className="feature-workflow-copy">
							<span className="feature-workflow-index">
								0{index + 1} / {t(`${flow.key}Label`)}
							</span>
							<h2>{t(`${flow.key}Title`)}</h2>
							<p>{t(`${flow.key}Body`)}</p>
							<ul>
								{[1, 2, 3].map((item) => (
									<li key={item}>
										<Check size={15} />
										<span>{t(`${flow.key}Item${item}`)}</span>
									</li>
								))}
							</ul>
							<Link className="route-text-link" href={flow.href}>
								{c("viewDocs")}
								<ArrowUpRight size={15} />
							</Link>
						</div>
						<figure className="feature-workflow-figure">
							<ThemedShot
								name={flow.image}
								alt={t(`${flow.key}Alt`)}
								width={1920}
								height={1200}
								sizes="(max-width:900px) 100vw,55vw"
							/>
							<figcaption>
								<span aria-hidden="true">$</span>
								<code>{flow.command}</code>
							</figcaption>
						</figure>
					</PageReveal>
				))}
			</div>
		</section>
	);
}

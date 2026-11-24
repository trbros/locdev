import { ArrowUpRight, Eye, ShieldCheck, Undo2 } from "lucide-react";
import { getTranslations } from "next-intl/server";
import { PageReveal } from "@/components/page-reveal";
import { Link } from "@/i18n/navigation";
import { route } from "@/lib/routes.generated";

export async function FeatureControl() {
	const t = await getTranslations("FeatureDetails");
	const controls = [
		{ icon: Eye, key: "inspect" },
		{ icon: ShieldCheck, key: "approve" },
		{ icon: Undo2, key: "undo" },
	] as const;
	return (
		<>
			<section
				id="control"
				className="route-section feature-control border-y border-border"
			>
				<div className="page-container">
					<div className="route-section-heading">
						<div>
							<p className="section-eyebrow">{t("controlEyebrow")}</p>
							<h2>{t("controlTitle")}</h2>
						</div>
						<Link
							href={route("docs.$slug", { slug: "security" })}
							className="route-text-link"
						>
							{t("securityLink")}
							<ArrowUpRight size={15} />
						</Link>
					</div>
					<PageReveal className="feature-control-grid">
						{controls.map((item) => (
							<article key={item.key}>
								<item.icon size={24} />
								<h3>{t(`${item.key}Title`)}</h3>
								<p>{t(`${item.key}Body`)}</p>
							</article>
						))}
					</PageReveal>
				</div>
			</section>
			<section className="route-section">
				<PageReveal className="page-container feature-next">
					<div>
						<p className="section-eyebrow">{t("nextEyebrow")}</p>
						<h2>{t("nextTitle")}</h2>
						<p>{t("nextBody")}</p>
					</div>
					<div className="feature-next-actions">
						<Link
							className="btn-ember"
							href={route("docs.$slug", { slug: "getting-started" })}
						>
							{t("startLink")}
							<ArrowUpRight size={16} />
						</Link>
						<Link className="btn-outline" href="/download">
							{t("downloadLink")}
							<ArrowUpRight size={16} />
						</Link>
					</div>
				</PageReveal>
			</section>
		</>
	);
}

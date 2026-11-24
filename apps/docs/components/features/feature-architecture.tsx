import {
	AppWindow,
	ArrowRight,
	ArrowUpRight,
	Cpu,
	Network,
	Terminal,
} from "lucide-react";
import { getTranslations } from "next-intl/server";
import { PageReveal } from "@/components/page-reveal";
import { Link } from "@/i18n/navigation";
import { route } from "@/lib/routes.generated";

export async function FeatureArchitecture() {
	const t = await getTranslations("FeaturePage");
	const c = await getTranslations("Common");
	const layers = [
		{ icon: AppWindow, title: t("desktop"), body: t("desktopBody") },
		{ icon: Terminal, title: t("cli"), body: t("cliBody") },
		{ icon: Cpu, title: t("daemon"), body: t("daemonBody") },
		{ icon: Network, title: t("network"), body: t("networkBody") },
	];
	return (
		<section id="architecture" className="route-section border-t border-border">
			<div className="page-container">
				<div className="route-section-heading">
					<div>
						<p className="section-eyebrow">{t("architectureEyebrow")}</p>
						<h2>{t("architectureTitle")}</h2>
					</div>
					<Link
						href={route("docs.$slug", { slug: "architecture" })}
						className="route-text-link"
					>
						{c("viewDocs")}
						<ArrowUpRight size={15} />
					</Link>
				</div>
				<PageReveal className="architecture-flow">
					{layers.map((layer, index) => (
						<article className="architecture-step" key={layer.title}>
							<span className="architecture-step-icon">
								<layer.icon size={22} />
							</span>
							<h3>{layer.title}</h3>
							<p>{layer.body}</p>
							{index === 0 ? (
								<span className="architecture-arrow" aria-hidden="true">
									+
								</span>
							) : (
								index < layers.length - 1 && (
									<ArrowRight
										className="architecture-arrow"
										size={16}
										aria-hidden="true"
									/>
								)
							)}
						</article>
					))}
				</PageReveal>
			</div>
		</section>
	);
}

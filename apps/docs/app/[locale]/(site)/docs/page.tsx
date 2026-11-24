import { ArrowRight, ArrowUpRight, ChevronRight, LifeBuoy } from "lucide-react";
import { getTranslations } from "next-intl/server";
import { docIcons } from "@/components/docs/doc-icons";
import { DocsSidebar } from "@/components/docs/docs-sidebar";
import { Screenshot } from "@/components/docs/screenshot";
import { Link } from "@/i18n/navigation";
import { docGroups } from "@/lib/docs";
import { getDocLabels } from "@/lib/docs-labels";
import { route } from "@/lib/routes.generated";

export default async function DocsIndex({
	params,
}: {
	params: Promise<{ locale: string }>;
}) {
	const { locale } = await params;
	const t = await getTranslations("Docs");
	const labels = getDocLabels(locale);
	const support = await getTranslations("Support");
	return (
		<div className="page-container docs-detail-shell docs-index-shell">
			<DocsSidebar locale={locale} />
			<div className="docs-index-content">
				<section className="docs-index-hero">
					<div className="docs-breadcrumb">
						<Link href="/">LocDev</Link>
						<ChevronRight size={13} />
						<span>{t("title")}</span>
					</div>
					<h1>{t("title")}</h1>
					<p>{t("body")}</p>
					<div className="docs-quickstarts">
						<Link
							href={route("docs.$slug", { slug: "getting-started" })}
							className="btn-ember"
						>
							{labels.titles["getting-started"]}
							<ArrowRight size={16} />
						</Link>
						<Link
							href={route("docs.$slug", { slug: "app" })}
							className="btn-outline"
						>
							{labels.titles.app}
							<ArrowUpRight size={16} />
						</Link>
					</div>
				</section>
				<div className="docs-landing">
					{docGroups.map((group) => (
						<section key={group.id} className="docs-group">
							<header className="docs-group-heading">
								<h2>{labels.groups[group.id]}</h2>
							</header>
							<div className="docs-link-grid">
								{group.slugs.map((slug) => {
									const Icon = docIcons[slug];
									return (
										<Link
											href={route("docs.$slug", { slug: slug })}
											key={slug}
											className="docs-link-card"
										>
											<Icon
												size={21}
												className="docs-card-icon"
												aria-hidden="true"
											/>
											<strong>{labels.titles[slug]}</strong>
											<span className="text-muted-foreground text-sm leading-6">
												{labels.descriptions[slug]}
											</span>
											<ArrowUpRight size={17} className="docs-link-arrow" />
										</Link>
									);
								})}
							</div>
						</section>
					))}
					<Link href="/support" className="docs-help">
						<LifeBuoy size={22} />
						<span>
							<strong>{support("title")}</strong>
							<small>{labels.descriptions.troubleshooting}</small>
						</span>
						<ArrowRight size={18} />
					</Link>
					<Link
						href={route("docs.$slug", { slug: "app" })}
						className="docs-tour"
					>
						<Screenshot name="sites" alt={labels.descriptions.app} />
						<span>
							{labels.titles.app}
							<ArrowRight size={16} />
						</span>
					</Link>
				</div>
			</div>
		</div>
	);
}

import {
	ArrowRight,
	ArrowUpRight,
	Cpu,
	GitBranch,
	Mail,
	ShieldCheck,
} from "lucide-react";
import type { Metadata } from "next";
import { getTranslations } from "next-intl/server";
import { Suspense } from "react";
import { ComparisonTable } from "@/components/comparison/comparison-table";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
} from "@/components/ui/accordion";
import { Link } from "@/i18n/navigation";
import { reviewedOn } from "@/lib/comparison-matrix";
import "@/components/comparison/comparison.css";
import { route } from "@/lib/routes.generated";

export async function generateMetadata(): Promise<Metadata> {
	const t = await getTranslations("ComparisonMatrix.page");
	return {
		title: `${t("metaTitle")} | LocDev`,
		description: t("metaDescription"),
	};
}

export default async function ComparePage() {
	const t = await getTranslations("ComparisonMatrix");
	const reasons = [
		{ id: "native", icon: Cpu },
		{ id: "git", icon: GitBranch },
		{ id: "email", icon: Mail },
		{ id: "debug", icon: ShieldCheck },
	];
	return (
		<main className="page-container compare-page">
			<header className="compare-hero">
				<div>
					<p className="section-eyebrow">{t("page.eyebrow")}</p>
					<h1>
						{t("experience.title")}
						<span className="compare-hero-accent">
							{t("experience.titleAccent")}
						</span>
					</h1>
				</div>
				<div className="compare-hero-aside">
					<p>{t("experience.intro")}</p>
					<Link href={route("docs.$slug", { slug: "getting-started" })}>
						{t("experience.startAction")}
						<ArrowRight size={16} aria-hidden="true" />
					</Link>
				</div>
			</header>

			<Suspense
				fallback={
					<div className="compare-pending" role="status">
						{t("experience.loading")}
					</div>
				}
			>
				<ComparisonTable />
			</Suspense>

			<section
				className="compare-reasons"
				aria-labelledby="compare-reasons-title"
			>
				<div className="compare-section-intro">
					<p className="section-eyebrow">{t("why.eyebrow")}</p>
					<h2 id="compare-reasons-title">{t("why.title")}</h2>
					<p>{t("why.body")}</p>
				</div>
				<div className="compare-reasons-grid">
					{reasons.map(({ id, icon: Icon }) => (
						<article key={id}>
							<Icon size={24} aria-hidden="true" />
							<h3>{t(`why.${id}Title`)}</h3>
							<p>{t(`why.${id}Body`)}</p>
						</article>
					))}
				</div>
			</section>

			<section
				className="compare-method"
				aria-labelledby="compare-method-title"
			>
				<div className="compare-section-intro">
					<p className="section-eyebrow">{t("experience.methodEyebrow")}</p>
					<h2 id="compare-method-title">{t("page.fairTitle")}</h2>
					<p>{t("page.fairBody")}</p>
				</div>
				<Accordion
					className="compare-method-questions"
					defaultValue={["statuses"]}
				>
					<AccordionItem value="statuses">
						<AccordionTrigger>{t("experience.readStatuses")}</AccordionTrigger>
						<AccordionContent>
							<p>{t("ui.disclaimer")}</p>
							<p>{t("page.fairDetails")}</p>
						</AccordionContent>
					</AccordionItem>
					<AccordionItem value="limits">
						<AccordionTrigger>{t("why.limitsTitle")}</AccordionTrigger>
						<AccordionContent>
							<p>{t("why.limitsBody")}</p>
							<p>{t("why.evidence")}</p>
						</AccordionContent>
					</AccordionItem>
					<AccordionItem value="correction">
						<AccordionTrigger>{t("page.correctionTitle")}</AccordionTrigger>
						<AccordionContent>
							<p>{t("page.correctionBody", { date: reviewedOn })}</p>
							<a
								href="https://github.com/trbros/locdev/issues/new"
								target="_blank"
								rel="noopener noreferrer"
							>
								{t("page.correctionAction")}
								<ArrowUpRight size={14} aria-hidden="true" />
							</a>
						</AccordionContent>
					</AccordionItem>
				</Accordion>
			</section>

			<section className="compare-cta">
				<div>
					<p className="section-eyebrow">{t("experience.ctaEyebrow")}</p>
					<h2>{t("experience.ctaTitle")}</h2>
					<p>{t("experience.ctaBody")}</p>
				</div>
				<div>
					<Link className="compare-primary-action" href="/download">
						{t("experience.downloadAction")}
						<ArrowRight size={17} aria-hidden="true" />
					</Link>
					<Link className="compare-secondary-action" href={route("docs")}>
						{t("page.docsAction")}
						<ArrowUpRight size={16} aria-hidden="true" />
					</Link>
				</div>
			</section>
		</main>
	);
}

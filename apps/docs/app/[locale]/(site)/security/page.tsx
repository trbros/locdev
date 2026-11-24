import {
	ArrowRight,
	ArrowUpRight,
	BookOpen,
	Check,
	FileText,
	LockKeyhole,
	Mail,
	ShieldCheck,
} from "lucide-react";
import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { getTranslations } from "next-intl/server";
import { PageReveal } from "@/components/page-reveal";
import { Link } from "@/i18n/navigation";
import { route } from "@/lib/routes.generated";
import { siteInformation } from "@/lib/site-information";

export async function generateMetadata(): Promise<Metadata> {
	const t = await getTranslations("SecurityReporting");
	return {
		title: t("title"),
		description: t("body"),
		robots: siteInformation.securityContact
			? undefined
			: { index: false, follow: false },
	};
}
export default async function SecurityPage() {
	if (!siteInformation.securityContact) notFound();
	const t = await getTranslations("SecurityReporting");
	return (
		<div className="page-container security-page">
			<header className="security-hero">
				<div>
					<p className="section-eyebrow">{t("eyebrow")}</p>
					<h1>{t("title")}</h1>
					<p className="route-description">{t("body")}</p>
					<div className="route-actions">
						<a className="btn-ember" href={siteInformation.securityContact}>
							<Mail size={16} />
							{t("action")}
							<ArrowUpRight size={15} />
						</a>
						<a href="#security-report-details" className="btn-outline">
							{t("includeTitle")}
							<ArrowRight size={15} />
						</a>
					</div>
				</div>
				<div className="security-emblem" aria-hidden="true">
					<div className="security-emblem-ring" />
					<div className="security-emblem-ring security-emblem-ring-inner" />
					<div className="security-emblem-core">
						<ShieldCheck size={72} strokeWidth={1.25} />
					</div>
					<span className="security-emblem-label">locdev / security</span>
				</div>
			</header>
			<PageReveal className="security-content">
				<section
					className="security-private-channel"
					aria-labelledby="security-channel-title"
				>
					<div className="security-section-icon">
						<LockKeyhole size={22} aria-hidden="true" />
					</div>
					<div>
						<h2 id="security-channel-title">{t("privateTitle")}</h2>
						<p>
							{t("privateBody", {
								contact: siteInformation.securityContact.replace(
									/^mailto:/,
									"",
								),
							})}
						</p>
						<a
							className="security-contact-link"
							href={siteInformation.securityContact}
						>
							<Mail size={16} aria-hidden="true" />
							<span>
								{siteInformation.securityContact.replace(/^mailto:/, "")}
							</span>
							<ArrowUpRight size={15} aria-hidden="true" />
						</a>
					</div>
				</section>
				<section
					className="security-report-details"
					id="security-report-details"
					aria-labelledby="security-details-title"
				>
					<div className="security-section-heading">
						<p className="section-eyebrow">{t("prepare")}</p>
						<h2 id="security-details-title">{t("includeTitle")}</h2>
						<p>{t("prepareBody")}</p>
					</div>
					<ol className="security-report-checklist">
						{["step1", "step2", "step3"].map((key, index) => (
							<li key={key}>
								<span className="security-step-number" aria-hidden="true">
									{String(index + 1).padStart(2, "0")}
								</span>
								<div>
									<h3>{t(`stepTitle${index + 1}`)}</h3>
									<p>{t(key)}</p>
								</div>
								<Check size={16} aria-hidden="true" />
							</li>
						))}
					</ol>
				</section>
				<div className="security-process-grid">
					<section aria-labelledby="security-process-title">
						<FileText size={24} aria-hidden="true" />
						<h2 id="security-process-title">{t("processTitle")}</h2>
						<p>{t("process")}</p>
					</section>
					<section aria-labelledby="security-limits-title">
						<ShieldCheck size={24} aria-hidden="true" />
						<h2 id="security-limits-title">{t("limitsTitle")}</h2>
						<p>{t("limits")}</p>
					</section>
				</div>
				<section
					className="security-resources"
					aria-labelledby="security-resources-title"
				>
					<div>
						<BookOpen size={22} aria-hidden="true" />
						<h2 id="security-resources-title">{t("resourcesTitle")}</h2>
					</div>
					<nav aria-labelledby="security-resources-title">
						<Link href={route("docs.$slug", { slug: "security" })}>
							{t("docs")}
							<ArrowRight size={15} />
						</Link>
						<Link href={route("docs.$slug", { slug: "https" })}>
							HTTPS
							<ArrowRight size={15} />
						</Link>
						<Link href="/support">
							{t("supportAction")}
							<ArrowRight size={15} />
						</Link>
					</nav>
				</section>
			</PageReveal>
		</div>
	);
}

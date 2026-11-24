import {
	ArrowRight,
	ArrowUpRight,
	BookOpen,
	Bug,
	Download,
	FolderOpen,
	Globe,
	HeartPulse,
	RefreshCw,
	ShieldCheck,
	Terminal,
} from "lucide-react";
import type { Metadata } from "next";
import { getTranslations } from "next-intl/server";
import { CodeBlock } from "@/components/docs/code-block";
import { HighlightedCode } from "@/components/docs/highlighted-code";
import { PageReveal } from "@/components/page-reveal";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
} from "@/components/ui/accordion";
import { Link } from "@/i18n/navigation";
import { repository } from "@/lib/repository";
import { route } from "@/lib/routes.generated";
import { siteInformation } from "@/lib/site-information";
export async function generateMetadata(): Promise<Metadata> {
	const t = await getTranslations("Support");
	return { title: t("title"), description: t("body") };
}
export default async function SupportPage() {
	const t = await getTranslations("Support");
	const report = `${repository.issues}?${new URLSearchParams({ title: "Bug report", body: "## LocDev version\n\n## OS and architecture\n\n## Stack and command\n\n## Steps to reproduce\n\n## Expected behavior\n\n## Actual behavior\n\n## Redacted logs (no tokens, passwords or personal data)\n" })}`;
	return (
		<div className="page-container support-page">
			<header className="support-hero">
				<div>
					<p className="section-eyebrow">{t("eyebrow")}</p>
					<h1 className="section-heading">{t("title")}</h1>
					<p className="route-description">{t("body")}</p>
					<div className="route-actions">
						<Link href="#support-topics" className="btn-ember">
							{t("findAnswer")}
							<ArrowRight size={15} />
						</Link>
						<a
							href={report}
							className="btn-outline"
							target="_blank"
							rel="noopener noreferrer"
						>
							{t("reportAction")}
							<ArrowUpRight size={15} />
						</a>
					</div>
				</div>
				<div className="support-hero-aside">
					<Terminal size={24} aria-hidden="true" />
					<p>{t("quickCheck")}</p>
					<CodeBlock code="locdev doctor" language="bash">
						<HighlightedCode code="locdev doctor" language="bash" />
					</CodeBlock>
					<Link href="#support-diagnostics">
						{t("doctorTitle")}
						<ArrowRight size={14} />
					</Link>
				</div>
			</header>
			<PageReveal className="support-entry-links">
				{[
					{ key: "docs", path: route("docs"), icon: BookOpen },
					{
						key: "doctor",
						path: route("docs.$slug", { slug: "doctor" }),
						icon: HeartPulse,
					},
					{ key: "install", path: "/get-started", icon: Download },
				].map(({ key, path, icon: Icon }) => (
					<Link href={path} key={key}>
						<Icon size={22} aria-hidden="true" />
						<h2>{t(`${key}Title`)}</h2>
						<p>{t(`${key}Body`)}</p>
						<ArrowRight size={16} aria-hidden="true" />
					</Link>
				))}
			</PageReveal>
			<section
				id="support-topics"
				className="support-topics"
				aria-labelledby="support-topics-title"
			>
				<div className="support-section-heading">
					<p className="section-eyebrow">{t("topicsEyebrow")}</p>
					<h2 id="support-topics-title">{t("topicsTitle")}</h2>
					<p>{t("topicsBody")}</p>
				</div>
				<Accordion className="support-topic-list">
					{[
						{ key: "installation", icon: Download, href: "/get-started" },
						{
							key: "projects",
							icon: FolderOpen,
							href: route("docs.$slug", { slug: "sites" }),
						},
						{
							key: "domains",
							icon: Globe,
							href: route("docs.$slug", { slug: "domains" }),
						},
						{
							key: "https",
							icon: ShieldCheck,
							href: route("docs.$slug", { slug: "https" }),
						},
						{
							key: "runtime",
							icon: Terminal,
							href: route("docs.$slug", { slug: "runtimes" }),
						},
						{
							key: "updates",
							icon: RefreshCw,
							href: route("docs.$slug", { slug: "updating" }),
						},
					].map(({ key, icon: Icon, href }) => (
						<AccordionItem key={key} value={key}>
							<AccordionTrigger>
								<span className="support-topic-title">
									<Icon size={18} aria-hidden="true" />
									{t(`topics.${key}.title`)}
								</span>
							</AccordionTrigger>
							<AccordionContent>
								<p>{t(`topics.${key}.body`)}</p>
								<Link href={href} className="tool-text-link">
									{t("openGuide")}
									<ArrowRight size={14} />
								</Link>
							</AccordionContent>
						</AccordionItem>
					))}
				</Accordion>
			</section>
			<section id="support-diagnostics" className="support-diagnostic-panel">
				<div>
					<h2>{t("checkTitle")}</h2>
					<p>{t("checkBody")}</p>
					<ol>
						{["step1", "step2", "step3", "step4"].map((key) => (
							<li key={key}>{t(key)}</li>
						))}
					</ol>
				</div>
				<div className="support-diagnostic-commands">
					<p className="support-command-label">{t("diagnosticCommands")}</p>
					<CodeBlock
						code={"locdev --version\nlocdev status\nlocdev doctor"}
						language="bash"
					>
						<HighlightedCode
							code={"locdev --version\nlocdev status\nlocdev doctor"}
							language="bash"
						/>
					</CodeBlock>
					<Link
						href={route("docs.$slug", { slug: "doctor" })}
						className="tool-text-link"
					>
						{t("doctorGuide")}
						<ArrowRight size={14} />
					</Link>
				</div>
			</section>
			<section className="support-report-options">
				<div>
					<Bug size={23} />
					<h2>{t("reportTitle")}</h2>
					<p>{t("reportBody")}</p>
					<a
						className="btn-ember"
						href={report}
						target="_blank"
						rel="noopener noreferrer"
					>
						{t("reportAction")}
						<ArrowRight size={15} />
					</a>
					<p className="support-response-note">{t("noSla")}</p>
				</div>
				<div>
					<ShieldCheck size={23} />
					<h2>{t("securityTitle")}</h2>
					<p>{t("securityBody")}</p>
					{siteInformation.securityContact ? (
						<Link href="/security" className="btn-outline">
							{t("privateReport")}
						</Link>
					) : (
						<p className="support-response-note">{t("securityPending")}</p>
					)}
					<Link href={route("docs.$slug", { slug: "security" })}>
						{t("securityDocs")}
					</Link>
				</div>
			</section>
			<div className="support-resources">
				<section className="support-resource-section">
					<h2>{t("maintenanceTitle")}</h2>
					<div className="support-resource-links">
						<Link href={route("docs.$slug", { slug: "troubleshooting" })}>
							{t("troubleshooting")}
						</Link>
						<Link href={route("docs.$slug", { slug: "updating" })}>
							{t("updating")}
						</Link>
						<Link href={route("docs.$slug", { slug: "uninstalling" })}>
							{t("uninstalling")}
						</Link>
					</div>
				</section>
				<section className="support-resource-section">
					<h2>{t("knownTitle")}</h2>
					<p>{t("knownBody")}</p>
					<div className="support-resource-links">
						<a
							href={`${repository.url}/issues`}
							target="_blank"
							rel="noopener noreferrer"
						>
							{t("issues")}
						</a>
						<Link href="/changelog">{t("changelog")}</Link>
						<Link href="/download">{t("requirements")}</Link>
					</div>
				</section>
			</div>
		</div>
	);
}

import {
	ArrowDown,
	ArrowUpRight,
	BookOpen,
	Code2,
	Coffee,
	FlaskConical,
	GitBranch,
	Heart,
	PackageCheck,
	ShieldCheck,
	Sparkles,
	Terminal,
} from "lucide-react";
import type { Metadata } from "next";
import { getTranslations } from "next-intl/server";
import { PageReveal } from "@/components/page-reveal";
import { Link } from "@/i18n/navigation";
import { githubRepo, sponsors } from "@/lib/project";
import { route } from "@/lib/routes.generated";

export async function generateMetadata(): Promise<Metadata> {
	const t = await getTranslations("Donate");
	return { title: t("label"), description: t("body") };
}
export default async function DonatePage() {
	const t = await getTranslations("Donate");
	const priorities = [
		{
			key: "maintenance",
			icon: ShieldCheck,
			href: route("docs.$slug", { slug: "architecture" }),
		},
		{ key: "experience", icon: Sparkles, href: "/features" },
		{
			key: "runtimes",
			icon: Terminal,
			href: route("docs.$slug", { slug: "runtimes" }),
		},
		{
			key: "releases",
			icon: PackageCheck,
			href: route("docs.$slug", { slug: "installation" }),
		},
	] as const;
	const ways = [
		{ key: "test", icon: FlaskConical, href: "/download" },
		{ key: "docs", icon: BookOpen, href: route("docs") },
		{ key: "code", icon: Code2, href: githubRepo },
	] as const;
	return (
		<div className="donate-page">
			<section className="route-hero page-container">
				<PageReveal className="route-hero-grid">
					<div className="route-hero-copy">
						<p className="section-eyebrow">{t("eyebrow")}</p>
						<h1>{t("title")}</h1>
						<p className="route-description">{t("body")}</p>
						<div className="route-actions">
							<a href="#priorities" className="btn-ember">
								{t("whyLink")}
								<ArrowDown size={16} />
							</a>
							<a href="#other-ways" className="btn-outline">
								{t("otherWays")}
								<ArrowUpRight size={16} />
							</a>
						</div>
					</div>
					<div className="donation-status">
						<div className="donation-heart" aria-hidden="true">
							<Heart size={44} strokeWidth={1.3} />
						</div>
						<span className="donation-open">
							<i />
							{t("status")}
						</span>
						<h2>{t("fundTitle")}</h2>
						<p>{t("fundBody")}</p>
						<a
							href={sponsors.github}
							target="_blank"
							rel="noopener noreferrer"
							className="btn-ember donation-participate"
						>
							<Heart size={15} />
							{t("sponsorsButton")}
							<ArrowUpRight size={15} />
						</a>
						<a
							href={sponsors.coffee}
							target="_blank"
							rel="noopener noreferrer"
							className="btn-outline donation-coffee"
						>
							<Coffee size={15} />
							{t("coffeeButton")}
							<ArrowUpRight size={15} />
						</a>
						<small>{t("noPayment")}</small>
					</div>
				</PageReveal>
			</section>
			<section id="priorities" className="route-section border-t border-border">
				<div className="page-container">
					<div className="route-section-heading">
						<div>
							<p className="section-eyebrow">{t("prioritiesEyebrow")}</p>
							<h2>{t("prioritiesTitle")}</h2>
						</div>
						<Link href="/changelog" className="route-text-link">
							{t("roadmapLink")}
							<ArrowUpRight size={15} />
						</Link>
					</div>
					<p className="donation-intro">{t("prioritiesBody")}</p>
					<PageReveal className="donation-priorities">
						{priorities.map((priority) => (
							<article key={priority.key}>
								<priority.icon size={26} />
								<h3>{t(`${priority.key}Title`)}</h3>
								<p>{t(`${priority.key}Body`)}</p>
								<Link href={priority.href} className="route-text-link">
									{t("explore")}
									<ArrowUpRight size={14} />
								</Link>
							</article>
						))}
					</PageReveal>
				</div>
			</section>
			<section className="route-section donation-principles border-y border-border">
				<PageReveal className="page-container donation-principles-layout">
					<div>
						<p className="section-eyebrow">{t("principlesEyebrow")}</p>
						<h2 className="route-heading">{t("principlesTitle")}</h2>
					</div>
					<div>
						<p>{t("voluntary")}</p>
						<p>{t("plans")}</p>
						<Link href="/license" className="route-text-link">
							{t("licenseLink")}
							<ArrowUpRight size={15} />
						</Link>
					</div>
				</PageReveal>
			</section>
			<section id="other-ways" className="route-section">
				<div className="page-container">
					<div className="route-section-heading">
						<div>
							<p className="section-eyebrow">{t("otherEyebrow")}</p>
							<h2>{t("otherTitle")}</h2>
						</div>
						<GitBranch size={24} className="text-ember" />
					</div>
					<PageReveal className="donation-ways">
						{ways.map((way) => (
							<article key={way.key}>
								<way.icon size={24} />
								<h3>{t(`${way.key}Title`)}</h3>
								<p>{t(`${way.key}Body`)}</p>
								{way.href.startsWith("https") ? (
									<a
										href={way.href}
										target="_blank"
										rel="noopener noreferrer"
										className="route-text-link"
									>
										{t(`${way.key}Link`)}
										<ArrowUpRight size={14} />
									</a>
								) : (
									<Link href={way.href} className="route-text-link">
										{t(`${way.key}Link`)}
										<ArrowUpRight size={14} />
									</Link>
								)}
							</article>
						))}
					</PageReveal>
					<p className="route-footnote">{t("sourceNote")}</p>
				</div>
			</section>
		</div>
	);
}

import {
	ArrowUpRight,
	Bug,
	Code2,
	FileText,
	Heart,
	Languages,
	Puzzle,
} from "lucide-react";
import type { Metadata } from "next";
import { getTranslations } from "next-intl/server";
import { CodeBlock } from "@/components/docs/code-block";
import { HighlightedCode } from "@/components/docs/highlighted-code";
import { PageReveal } from "@/components/page-reveal";
import { Link } from "@/i18n/navigation";
import { repository } from "@/lib/repository";
import { frameworkRequestUrl } from "@/lib/stacks";
export async function generateMetadata(): Promise<Metadata> {
	const t = await getTranslations("Contribute");
	return { title: t("title"), description: t("body") };
}
function issue(kind: string, body: string) {
	return `${repository.issues}?${new URLSearchParams({ title: kind, body })}`;
}
export default async function ContributePage() {
	const t = await getTranslations("Contribute");
	const ways = [
		{ key: "code", icon: Code2, url: repository.pulls },
		{
			key: "bugs",
			icon: Bug,
			url: issue(
				"Bug report",
				"## LocDev version\n\n## OS and architecture\n\n## Steps to reproduce\n\n## Expected and actual behavior\n\n## Relevant logs (remove personal information and secrets)\n",
			),
		},
		{ key: "frameworks", icon: Puzzle, url: frameworkRequestUrl() },
		{
			key: "docs",
			icon: FileText,
			url: issue(
				"Documentation improvement",
				"## Documentation page\n\n## What needs to change\n\n## Suggested correction\n",
			),
		},
		{
			key: "translations",
			icon: Languages,
			url: issue(
				"Translation improvement",
				"## Language\n\n## Page URL\n\n## Current text\n\n## Suggested translation\n",
			),
		},
	];
	const checks =
		"cargo fmt --all --check\ncargo clippy --all-targets -- -D warnings\ncargo test";
	return (
		<div className="page-container tool-page contribute-page">
			<header className="route-hero tool-hero">
				<p className="section-eyebrow">{t("eyebrow")}</p>
				<h1 className="section-heading">{t("title")}</h1>
				<p className="route-description">{t("body")}</p>
				<div className="route-actions">
					<a href="#contribution-paths" className="btn-ember">
						{t("startTitle")}
						<ArrowUpRight size={15} />
					</a>
					<a
						href={repository.url}
						className="btn-outline"
						target="_blank"
						rel="noopener noreferrer"
					>
						GitHub
						<ArrowUpRight size={15} />
					</a>
				</div>
			</header>
			<div id="contribution-paths">
				<PageReveal className="contribution-grid">
					{ways.map((way) => (
						<a
							key={way.key}
							className="contribution-card"
							href={way.url}
							target="_blank"
							rel="noopener noreferrer"
						>
							<way.icon size={26} />
							<h2>{t(`${way.key}Title`)}</h2>
							<p>{t(`${way.key}Body`)}</p>
							<span>
								{t("openGithub")}
								<ArrowUpRight size={15} />
							</span>
						</a>
					))}
					<Link href="/donate" className="contribution-card">
						<Heart size={26} />
						<h2>{t("fundingTitle")}</h2>
						<p>{t("fundingBody")}</p>
						<span>
							{t("viewFunding")}
							<ArrowUpRight size={15} />
						</span>
					</Link>
				</PageReveal>
			</div>
			<section className="contribute-start">
				<div>
					<p className="section-eyebrow">{t("startEyebrow")}</p>
					<h2>{t("startTitle")}</h2>
					<ol>
						{["step1", "step2", "step3", "step4"].map((key) => (
							<li key={key}>{t(key)}</li>
						))}
					</ol>
					<p>{t("reviewNote")}</p>
				</div>
				<div>
					<CodeBlock
						code={`git clone ${repository.clone} locdev\ncd locdev`}
						language="bash"
					/>{" "}
					<h3>{t("checks")}</h3>
					<CodeBlock code={checks} language="bash">
						<HighlightedCode code={checks} language="bash" />
					</CodeBlock>
					<p className="tool-caption">{t("ciNote")}</p>
				</div>
			</section>
		</div>
	);
}

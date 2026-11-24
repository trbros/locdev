import { ArrowDownToLine, ArrowUpRight } from "lucide-react";
import type { Metadata } from "next";
import { getLocale, getTranslations } from "next-intl/server";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { ScrollToHash } from "@/components/scroll-to-hash";
import { Link } from "@/i18n/navigation";
import { getChangelog } from "@/lib/changelog";
import { getReleaseFeed } from "@/lib/github-releases";
import { githubReleases } from "@/lib/repository";
export async function generateMetadata(): Promise<Metadata> {
	const t = await getTranslations("Changelog");
	return { title: t("title"), description: t("body") };
}
export default async function ChangelogPage() {
	const t = await getTranslations("Changelog");
	const r = await getTranslations("Releases");
	const feed = await getReleaseFeed();
	const entries = await getChangelog(await getLocale());
	return (
		<div className="page-container tool-page">
			<header className="route-hero tool-hero">
				<p className="section-eyebrow">{t("eyebrow")}</p>
				<h1 className="section-heading">{t("title")}</h1>
				<p className="route-description">{t("body")}</p>
				<div className="route-actions">
					<Link href="/download" className="btn-outline">
						{r("download")}
						<ArrowDownToLine size={15} />
					</Link>
					<a
						className="tool-text-link"
						href={githubReleases}
						target="_blank"
						rel="noopener noreferrer"
					>
						{t("github")}
						<ArrowUpRight size={14} />
					</a>
				</div>
			</header>
			<div className="changelog-list">
				{entries.map((entry, index) => {
					const previous = entries
						.slice(index + 1)
						.find((older) => older.version !== "Unreleased");
					const release = feed.releases.find(
						(r) => r.tag.replace(/^v/, "") === entry.version,
					);
					return (
						<article
							key={entry.version}
							className="changelog-entry"
							id={`v${entry.version}`}
						>
							<div className="changelog-meta">
								<span
									className="release-channel"
									data-preview={!entry.released}
								>
									{entry.version === "Unreleased"
										? t("unreleased")
										: entry.released || release
											? r("stable")
											: t("notPublished")}
								</span>
								{entry.date && <time>{entry.date}</time>}
							</div>
							<div>
								<h2>
									{entry.version === "Unreleased"
										? t("unreleased")
										: entry.version}
								</h2>
								{previous && (
									<p className="changelog-since">
										{t("since", { version: `v${previous.version}` })}
									</p>
								)}
								<div className="release-notes">
									<ReactMarkdown
										remarkPlugins={[remarkGfm]}
										skipHtml
										components={{
											h3: ({ children }) => <h3>{children}</h3>,
											a: ({ children, href }) => (
												<a
													href={href}
													target="_blank"
													rel="noopener noreferrer"
												>
													{children}
												</a>
											),
										}}
									>
										{entry.body}
									</ReactMarkdown>
								</div>
								{release && (
									<div className="route-actions">
										<a
											href={release.url}
											className="btn-outline"
											target="_blank"
											rel="noopener noreferrer"
										>
											{t("fullRelease")}
											<ArrowUpRight size={14} />
										</a>
										<Link href="/download" className="tool-text-link">
											{r("download")}
											<ArrowDownToLine size={14} />
										</Link>
									</div>
								)}
							</div>
						</article>
					);
				})}
			</div>
			<p className="tool-caption">{t("source")}</p>
			<ScrollToHash />
		</div>
	);
}

import { ArrowDownToLine, ArrowRight, ArrowUpRight, Check } from "lucide-react";
import type { Metadata } from "next";
import { getTranslations } from "next-intl/server";
import { LogoMark } from "@/components/brand";
import { CodeBlock } from "@/components/docs/code-block";
import { HighlightedCode } from "@/components/docs/highlighted-code";
import { Apple, Linux, Windows } from "@/components/icons";
import { PageReveal } from "@/components/page-reveal";
import { DownloadPicker } from "@/components/releases/download-picker";
import { Link } from "@/i18n/navigation";
import { getReleaseFeed } from "@/lib/github-releases";
import { repository } from "@/lib/repository";
import { route } from "@/lib/routes.generated";

export async function generateMetadata(): Promise<Metadata> {
	const t = await getTranslations("Download");
	return { title: t("title"), description: t("body") };
}
export default async function DownloadPage() {
	const t = await getTranslations("Download");
	const c = await getTranslations("Common");
	const feed = await getReleaseFeed();
	const source = `git clone ${repository.clone} locdev\ncd locdev\ncargo build --release\ncd apps/desktop\npnpm install\npnpm tauri build`;
	return (
		<div className="download-page">
			<section className="route-hero page-container">
				<PageReveal className="route-hero-grid">
					<div className="route-hero-copy">
						<p className="section-eyebrow">{t("eyebrow")}</p>
						<h1>{t("title")}</h1>
						<p className="route-description">{t("body")}</p>
						<div className="route-actions">
							<Link href="#platforms" className="btn-ember">
								{t("osTitle")}
								<ArrowDownToLine size={16} />
							</Link>
							<Link
								href={route("docs.$slug", { slug: "installation" })}
								className="btn-outline"
							>
								{c("viewDocs")}
								<ArrowUpRight size={16} />
							</Link>
						</div>
					</div>
					<div className="install-visual" aria-hidden="true">
						<div className="install-orbit" />
						<div className="install-orbit install-orbit-outer" />
						<div className="install-logo">
							<LogoMark className="size-16" />
						</div>
						<span className="install-chip install-chip-linux">
							<Linux className="install-platform-icon" /> Linux
						</span>
						<span className="install-chip install-chip-mac">
							<Apple className="install-platform-icon" /> macOS
						</span>
						<span className="install-chip install-chip-windows">
							<Windows className="install-platform-icon" /> Windows
						</span>
						<span className="install-caption">DESKTOP + CLI + DAEMON</span>
					</div>
				</PageReveal>
			</section>
			<section id="platforms" className="route-section border-t border-border">
				<div className="page-container">
					<DownloadPicker feed={feed} />
					<PageReveal className="install-steps">
						{[t("step1"), t("step2"), t("step3")].map((step, index) => (
							<div key={step}>
								<span>0{index + 1}</span>
								<p>{step}</p>
								<Check size={15} aria-hidden="true" />
							</div>
						))}
					</PageReveal>
				</div>
			</section>
			<section className="route-section route-source-section border-t border-border">
				<PageReveal className="page-container source-layout">
					<div>
						<p className="section-eyebrow">{t("sourceEyebrow")}</p>
						<h2 className="route-heading">{t("sourceTitle")}</h2>
						<p className="route-description">{t("sourceBody")}</p>
						<Link
							href={route("docs.$slug", { slug: "installation" })}
							className="route-text-link mt-7"
						>
							{t("sourceLink")}
							<ArrowRight size={16} />
						</Link>
						<div className="source-requirements">
							<span>Rust</span>
							<span>Node.js 20+</span>
							<span>pnpm</span>
							<span>Tauri</span>
						</div>
					</div>
					<div className="source-code">
						<CodeBlock code={source} language="bash">
							<HighlightedCode code={source} language="bash" />
						</CodeBlock>
					</div>
				</PageReveal>
			</section>
		</div>
	);
}

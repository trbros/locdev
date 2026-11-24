import {
	ArrowUpRight,
	Database,
	ExternalLink,
	Fingerprint,
	Server,
	ShieldCheck,
} from "lucide-react";
import type { Metadata } from "next";
import { getTranslations } from "next-intl/server";
import { Link } from "@/i18n/navigation";
import { route } from "@/lib/routes.generated";
import { privacyConfirmed, siteInformation } from "@/lib/site-information";
export async function generateMetadata(): Promise<Metadata> {
	const t = await getTranslations("Privacy");
	return {
		title: t("title"),
		description: t("body"),
		robots: privacyConfirmed ? undefined : { index: false, follow: true },
	};
}
export default async function PrivacyPage() {
	const t = await getTranslations("Privacy");
	return (
		<div className="page-container editorial-page privacy-page">
			<header className="route-hero">
				<p className="section-eyebrow">{t("eyebrow")}</p>
				<h1 className="section-heading">{t("title")}</h1>
				<p className="route-description">{t("body")}</p>
				{!privacyConfirmed && (
					<p className="editorial-notice">{t("pending")}</p>
				)}
			</header>
			<div className="privacy-layout">
				<nav className="privacy-outline" aria-label={t("title")}>
					{["browser", "generator", "external", "hosting", "app"].map((key) => (
						<a key={key} href={`#privacy-${key}`}>
							{t(`${key}Title`)}
						</a>
					))}
					<a href="#privacy-contact">{t("contactTitle")}</a>
				</nav>
				<div className="privacy-sections">
					{[
						{ key: "browser", icon: Fingerprint },
						{ key: "generator", icon: Database },
						{ key: "external", icon: ExternalLink },
						{ key: "hosting", icon: Server },
						{ key: "app", icon: ShieldCheck },
					].map(({ key, icon: Icon }) => (
						<section key={key} id={`privacy-${key}`}>
							<Icon size={22} aria-hidden="true" />
							<div>
								<h2>{t(`${key}Title`)}</h2>
								<p>{t(`${key}Body`)}</p>
								{key === "hosting" && (
									<dl>
										<dt>{t("provider")}</dt>
										<dd>{siteInformation.hosting ?? t("unconfirmed")}</dd>
										<dt>{t("retention")}</dt>
										<dd>
											{siteInformation.retention ?? t("retentionDefault")}
										</dd>
									</dl>
								)}
								{key === "hosting" && (
									<div className="editorial-inline-links">
										<a
											href="https://vercel.com/docs/logs/runtime"
											target="_blank"
											rel="noopener noreferrer"
										>
											{t("hostingDocs")}
										</a>
										<a
											href="https://vercel.com/legal/privacy-notice"
											target="_blank"
											rel="noopener noreferrer"
										>
											{t("hostingPrivacy")}
										</a>
									</div>
								)}
								{key === "app" && (
									<Link href={route("docs.$slug", { slug: "security" })}>
										{t("appDocs")}
									</Link>
								)}
							</div>
						</section>
					))}
				</div>
			</div>
			<section
				id="privacy-contact"
				className="editorial-checklist privacy-contact"
			>
				<h2>{t("contactTitle")}</h2>
				<p>{t("contactBody")}</p>
				{siteInformation.privacyContact ? (
					<a className="btn-outline" href={siteInformation.privacyContact}>
						{t("contactAction", {
							contact: siteInformation.privacyContact.replace(/^mailto:/, ""),
						})}
						<ArrowUpRight size={15} />
					</a>
				) : (
					<p className="editorial-notice">{t("contactPending")}</p>
				)}
				<p className="editorial-note">{t("changes")}</p>
			</section>
		</div>
	);
}

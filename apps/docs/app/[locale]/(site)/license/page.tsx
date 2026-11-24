import { readFile } from "node:fs/promises";
import { join } from "node:path";
import {
	ArrowDownToLine,
	ArrowUpRight,
	Check,
	Code2,
	Copy,
	Scale,
	ShieldCheck,
} from "lucide-react";
import type { Metadata } from "next";
import { getTranslations } from "next-intl/server";
import { LicenseReader } from "@/components/license/license-reader";
import { PageReveal } from "@/components/page-reveal";

export async function generateMetadata(): Promise<Metadata> {
	const t = await getTranslations("License");
	return { title: t("label"), description: t("body") };
}
export default async function LicensePage() {
	const t = await getTranslations("License");
	const text = await readFile(
		join(process.cwd(), "public/licenses/Apache-2.0.txt"),
		"utf8",
	);
	const permissions = [
		{ icon: Check, title: t("useTitle"), body: t("useBody") },
		{ icon: Code2, title: t("modifyTitle"), body: t("modifyBody") },
		{ icon: Copy, title: t("shareTitle"), body: t("shareBody") },
	];
	return (
		<>
			<section className="route-hero page-container">
				<PageReveal className="route-hero-grid">
					<div className="route-hero-copy">
						<p className="section-eyebrow">{t("eyebrow")}</p>
						<h1>{t("title")}</h1>
						<p className="route-description">{t("body")}</p>
						<div className="route-actions">
							<a
								href="/licenses/Apache-2.0.txt"
								download="Apache-2.0.txt"
								className="btn-ember"
							>
								{t("download")}
								<ArrowDownToLine size={16} />
							</a>
							<a
								href="https://www.apache.org/licenses/LICENSE-2.0"
								target="_blank"
								rel="noopener noreferrer"
								className="btn-outline"
							>
								{t("official")}
								<ArrowUpRight size={16} />
							</a>
						</div>
					</div>
					<div className="license-emblem">
						<Scale size={48} strokeWidth={1.2} aria-hidden="true" />
						<span>{t("spdx")}</span>
						<strong>Apache-2.0</strong>
						<small>Apache License · Version 2.0</small>
						<div className="license-emblem-line" />
					</div>
				</PageReveal>
			</section>
			<section className="route-section border-t border-border">
				<div className="page-container">
					<div className="route-section-heading">
						<h2>{t("permissionsTitle")}</h2>
						<ShieldCheck size={24} className="text-phosphor" />
					</div>
					<PageReveal className="license-permissions">
						{permissions.map((item) => (
							<article key={item.title}>
								<item.icon size={23} />
								<h3>{item.title}</h3>
								<p>{item.body}</p>
							</article>
						))}
					</PageReveal>
					<PageReveal className="license-conditions">
						<article>
							<h2>{t("conditionsTitle")}</h2>
							<ul>
								{[t("condition1"), t("condition2"), t("condition3")].map(
									(condition) => (
										<li key={condition}>
											<Check size={16} />
											<span>{condition}</span>
										</li>
									),
								)}
							</ul>
						</article>
						<article>
							<h2>{t("limitsTitle")}</h2>
							<p>{t("limitsBody")}</p>
							<p className="license-dependencies">{t("dependencies")}</p>
						</article>
					</PageReveal>
				</div>
			</section>
			<section className="route-section border-t border-border">
				<div className="page-container">
					<div className="route-section-heading">
						<div>
							<p className="section-eyebrow">APACHE-2.0</p>
							<h2>{t("fullTitle")}</h2>
							<p className="route-description">{t("fullBody")}</p>
						</div>
					</div>
					<LicenseReader text={text} />
				</div>
			</section>
		</>
	);
}
